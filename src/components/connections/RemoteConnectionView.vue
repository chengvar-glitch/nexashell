<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef, nextTick, watch, onActivated, onDeactivated, computed } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { SearchAddon } from '@xterm/addon-search';
import '@xterm/xterm/css/xterm.css';
import { useSessionStore } from '@/features/session';
import { sessionApi } from '@/features/session';
import { createLogger } from '@/core/utils/logger';
import { listen, UnlistenFn, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '@/features/settings';
import { attachMacWebKitIMESymbolFix } from '@/core/utils/terminal-input-fix';
import { themeManager } from '@/core/utils/theme-manager';
import {
  resolveTerminalTheme,
  toXtermTheme,
} from '@/core/terminal-themes';
import ServerDashboard from './ServerDashboard.vue';
import TunnelManagerPanel from '@/components/tunnel/TunnelManagerPanel.vue';
import { FolderTree, Waypoints } from 'lucide-vue-next';
import { openFileManagerWindow } from '@/features/window';
import { useI18n } from 'vue-i18n';
import {
  useRemotePath,
  normalizeRemotePath,
} from '@/composables/use-remote-path';

const logger = createLogger('REMOTE_CONNECTION_VIEW');
const { t } = useI18n();

const sessionStore = useSessionStore();
const settingsStore = useSettingsStore();

const showDashboard = ref(false);
const activeDashboardTab = ref<'system' | 'uploads' | null>('system');
const showTunnelPanel = ref(false);

import type { ServerStatus, UploadTask } from '@/core/types';

const statusHistory = shallowRef<ServerStatus[]>([]);
const MAX_HISTORY = 60;
// Always keep the most recent snapshot so state isn't lost while the panel is
// hidden, but only rebuild the chart history array while the dashboard is
// visible (avoids re-allocation + full SVG recompute every poll when closed).
const latestStatus = shallowRef<ServerStatus | null>(null);
let statusUnlisten: UnlistenFn | null = null;
let lastStatusSession = '';

const setupStatusListener = async () => {
  if (statusUnlisten) statusUnlisten();
  // Reset history only when the session changed (first mount or a new
  // session); re-activating the same tab keeps its existing buffer.
  if (lastStatusSession !== props.sessionId) {
    statusHistory.value = [];
    latestStatus.value = null;
    lastStatusSession = props.sessionId;
  }
  if (!props.sessionId) return;

  statusUnlisten = await listen<ServerStatus>(
    `ssh-status-${props.sessionId}`,
    event => {
      latestStatus.value = event.payload;
      if (!showDashboard.value) return;
      const next = statusHistory.value.concat(event.payload);
      statusHistory.value =
        next.length > MAX_HISTORY ? next.slice(-MAX_HISTORY) : next;
    }
  );
};

// When the sidebar is reopened, backfill the visible history from the latest
// snapshot so the charts don't start empty after being hidden.
watch(showDashboard, shown => {
  if (shown && latestStatus.value) {
    const next = statusHistory.value.concat(latestStatus.value);
    statusHistory.value =
      next.length > MAX_HISTORY ? next.slice(-MAX_HISTORY) : next;
  }
});

// Terminal configuration constants - Now acting as defaults or base
const LATENCY_THRESHOLD_MS = 100;

/**
 * Props interface
 * Note: serverName is stored in session.connectionParams, no need to pass separately
 */
interface Props {
  sessionId?: string;
  tabType?: string;
  ip?: string;
  port?: number;
  username?: string;
  password?: string;
  privateKeyPath?: string | null;
  keyPassphrase?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  sessionId: '',
  tabType: 'ssh',
  ip: '',
  port: 22,
  username: '',
  password: '',
  privateKeyPath: null,
  keyPassphrase: null,
});

const terminalRef = ref<HTMLElement>();
const isDragging = ref(false);

const {
  currentRemotePath,
  remoteHomeDir,
  lastKnownAbsolutePath,
  lastPathDetectionSource,
  hasOscPath,
  detectRemotePath,
} = useRemotePath();

/** Open the standalone file-manager window for this session. */
const openFileManager = () => {
  void openFileManagerWindow(props.sessionId);
};

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;
let disposeIMEFix: (() => void) | null = null;

// Upload task tracking

const uploadTasks = shallowRef<UploadTask[]>([]);

const addUploadTask = (fileName: string, direction: 'upload' | 'download' = 'upload'): string => {
  const id = `transfer-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  // Replace the array reference so the dashboard's `props.uploadTasks`
  // reference changes and the task list re-renders (in-place unshift keeps
  // the same reference, so the list never updates).
  uploadTasks.value = [
    {
      id,
      fileName,
      direction,
      status: 'pending',
      progress: 0,
      message: 'Preparing...',
      timestamp: Date.now(),
    },
    ...uploadTasks.value,
  ];
  return id;
};

const updateUploadTask = (id: string, updates: Partial<UploadTask>) => {
  let changed = false;
  const next = uploadTasks.value.map(task => {
    if (task.id === id) {
      changed = true;
      return { ...task, ...updates };
    }
    return task;
  });
  if (changed) {
    uploadTasks.value = next;
  }
};

// Remove a task from the list entirely (used when an upload is cancelled and
// its remote file is deleted, so the now-meaningless entry does not linger).
const removeUploadTask = (id: string) => {
  uploadTasks.value = uploadTasks.value.filter(task => task.id !== id);
};

const clearUploadTasks = () => {
  // Only clear completed and error tasks, keep in-flight tasks
  uploadTasks.value = uploadTasks.value.filter(
    task =>
      task.status === 'uploading' ||
      task.status === 'downloading' ||
      task.status === 'pending' ||
      task.status === 'paused'
  );
};

// ---------------------------------------------------------------------------
// Pre-upload confirmation dialog
// ---------------------------------------------------------------------------

interface UploadPreviewItem {
  localPath: string;
  fileName: string;
  remotePath: string;
}

interface UploadConfirmState {
  open: boolean;
  targetDir: string;
  files: UploadPreviewItem[];
}

const uploadConfirm = ref<UploadConfirmState>({
  open: false,
  targetDir: '',
  files: [],
});

/** Number of files queued for confirmation. */
const confirmCount = computed(() => uploadConfirm.value.files.length);

const closeUploadConfirm = () => {
  uploadConfirm.value.open = false;
  uploadConfirm.value.files = [];
};

/**
 * Resolve the final remote file path for a local file against a target
 * directory, applying home expansion, relative-path resolution, and
 * normalization. This has no side effects and is reused by both the
 * confirmation dialog and the actual upload.
 */
const resolveUploadTarget = (
  path: string,
  targetDirOverride?: string
): { fileName: string; remotePath: string } => {
  const fileName = path.split('/').pop() || path;
  let targetDir =
    targetDirOverride ?? currentRemotePath.value ?? '';

  // 1. Handle home expansion
  if (remoteHomeDir.value) {
    if (targetDir === '~') {
      targetDir = remoteHomeDir.value;
    } else if (targetDir.startsWith('~/')) {
      targetDir = targetDir.replace('~', remoteHomeDir.value);
    }
  }

  // Resolve relative paths against last known absolute path
  if (
    targetDir &&
    !targetDir.startsWith('/') &&
    lastKnownAbsolutePath.value
  ) {
    targetDir = normalizeRemotePath(
      `${lastKnownAbsolutePath.value}/${targetDir}`
    );
  }

  // 2. Robust path normalization
  if (!targetDir || targetDir === '.' || targetDir === '') {
    if (lastKnownAbsolutePath.value) {
      targetDir = lastKnownAbsolutePath.value;
    } else {
      targetDir = remoteHomeDir.value || '.';
    }
  }

  // 3. Build final SFTP path
  let remotePath = '';
  if (targetDir === '.') {
    remotePath = fileName;
  } else {
    const base = normalizeRemotePath(targetDir);
    remotePath = base.endsWith('/') ? `${base}${fileName}` : `${base}/${fileName}`;
  }

  return { fileName, remotePath };
};

// Drag and drop listeners
let unlistenDrag: UnlistenFn | null = null;
let unlistenDragEnter: UnlistenFn | null = null;
let unlistenDragLeave: UnlistenFn | null = null;
let unlistenUpload: UnlistenFn | null = null;
let unlistenDownload: UnlistenFn | null = null;
let pendingResizeTimer: ReturnType<typeof setTimeout> | null = null;
let handleTerminalContextMenu: ((e: MouseEvent) => void) | null = null;

interface UploadProgressPayload {
  taskId: string;
  sessionId: string;
  progress: number;
  uploadedBytes: number;
  totalBytes: number;
  status: 'uploading' | 'paused' | 'success' | 'error' | 'cancelled';
  message: string;
  speed: number;
  error?: string;
}

/**
 * Handle file drop - Show a confirmation dialog with the resolved target
 * path before any upload actually starts. The upload only begins once the
 * user confirms. Non-blocking: returns immediately, all processing happens
 * in the background.
 */
const handleFileDrop = (paths: string[]) => {
  // Show dashboard immediately and switch to uploads tab (stays visible once
  // the user confirms the upload).
  // Ensure terminal retains focus
  nextTick(() => {
    terminal?.focus();
  });

  // Detect path and build a preview of the resolved target in the background.
  (async () => {
    try {
      await detectRemotePath(() => terminal);
      const detectedPath = currentRemotePath.value;

      logger.info('Files dropped', {
        paths,
        targetPath: detectedPath,
        home: remoteHomeDir.value,
      });

      // Resolve each file's final remote path for the confirmation dialog.
      const files = paths.map(path => {
        const { fileName, remotePath } = resolveUploadTarget(path);
        return { localPath: path, fileName, remotePath };
      });

      uploadConfirm.value = {
        open: true,
        targetDir: detectedPath || currentRemotePath.value || '',
        files,
      };
    } catch (err) {
      logger.error('Failed to process dropped files', err);
    }
  })();
};

/** Confirm the pending uploads and start transferring them in the background. */
const confirmUploads = () => {
  const { files, targetDir } = uploadConfirm.value;
  closeUploadConfirm();
  showDashboard.value = true;
  activeDashboardTab.value = 'uploads';
  const effectiveDir = targetDir || currentRemotePath.value || '';
  files.forEach(file => {
    processFileUpload(file.localPath, effectiveDir);
  });
};

/**
 * Recompute the previewed remote paths when the user edits the target
 * directory in the confirmation dialog.
 */
const recomputeConfirmTargets = () => {
  const { targetDir } = uploadConfirm.value;
  uploadConfirm.value.files = uploadConfirm.value.files.map(file => {
    const { remotePath } = resolveUploadTarget(file.localPath, targetDir);
    return { ...file, remotePath };
  });
};

/**
 * Process a single file upload asynchronously without blocking
 * Fire-and-forget pattern: start upload in background and return immediately
 */
const processFileUpload = async (path: string, targetDirOverride?: string) => {
  const taskId = addUploadTask(path.split('/').pop() || path);

  // Prepare upload parameters in this async function
  // But do NOT await the actual upload - let it run in background

  try {
    const { fileName, remotePath } = resolveUploadTarget(path, targetDirOverride);

    updateUploadTask(taskId, {
      status: 'uploading',
      progress: 10,
      message: `Preparing upload...`,
      remotePath,
    });

    logger.info('Path resolution', {
      originalPath: currentRemotePath.value,
      resolvedPath: remotePath,
      detectionMethod: lastPathDetectionSource.value,
      fileName,
    });

    // Start upload in background
    // Backend will handle the streaming and emit progress events
    invoke('upload_file_sftp', {
      sessionId: props.sessionId,
      taskId,
      localPath: path,
      remotePath,
    }).catch(err => {
      const errorMessage =
        err instanceof Error
          ? err.message
          : typeof err === 'string'
            ? err
            : JSON.stringify(err);

      updateUploadTask(taskId, {
        status: 'error',
        progress: 0,
        message: `Failed to start: ${errorMessage}`,
        error: errorMessage,
      });

      logger.error('Failed to start upload', err);
    });

    // Return immediately without waiting for upload to complete
    logger.info('Upload queued in background', { taskId, fileName });
  } catch (err: unknown) {
    const errorMessage =
      err instanceof Error
        ? err.message
        : typeof err === 'string'
          ? err
          : JSON.stringify(err);

    updateUploadTask(taskId, {
      status: 'error',
      progress: 0,
      message: `Failed to prepare: ${errorMessage}`,
      error: errorMessage,
    });

    logger.error('Failed to prepare upload', err);
  }
};

// ---------------------------------------------------------------------------
// Upload control: pause / resume / cancel
// ---------------------------------------------------------------------------

/** Pause a running upload at its next chunk boundary. */
const pauseUploadTask = async (taskId: string) => {
  // Optimistically reflect the paused state immediately.
  updateUploadTask(taskId, {
    status: 'paused',
    message: t('upload.pausing'),
  });
  try {
    await invoke('pause_upload', {
      sessionId: props.sessionId,
      taskId,
    });
    updateUploadTask(taskId, {
      message: t('upload.paused'),
    });
  } catch (err) {
    logger.error('Failed to pause upload', err);
    updateUploadTask(taskId, {
      status:
        uploadTasks.value.find(t => t.id === taskId)?.status ?? 'uploading',
      message: t('upload.failedToPause'),
    });
  }
};

/** Resume a previously paused upload. */
const resumeUploadTask = async (taskId: string) => {
  updateUploadTask(taskId, {
    status: 'uploading',
    message: t('upload.resuming'),
  });
  try {
    await invoke('resume_upload', {
      sessionId: props.sessionId,
      taskId,
    });
  } catch (err) {
    logger.error('Failed to resume upload', err);
    updateUploadTask(taskId, {
      status: 'paused',
      message: t('upload.failedToResume'),
    });
  }
};

/** Cancel a running or paused upload/download task. */
const cancelUploadTask = async (taskId: string) => {
  const task = uploadTasks.value.find(t => t.id === taskId);
  const isDownload = task?.direction === 'download';
  // Optimistically remove the task so the list is clean instantly. If the
  // backend cancellation fails, surface an error entry so the user knows.
  removeUploadTask(taskId);
  try {
    await invoke(isDownload ? 'cancel_download' : 'cancel_upload', {
      sessionId: props.sessionId,
      taskId,
    });
  } catch (err) {
    logger.error('Failed to cancel transfer', err);
    const previous = uploadTasks.value.find(t => t.id === taskId);
    // Re-insert a terminal error entry (the cancelled-pending remote file may
    // be left behind if the backend cancellation truly failed).
    uploadTasks.value = [
      ...uploadTasks.value,
      previous ? { ...previous, status: 'error' as const, message: t('upload.failedToCancel') } : {
        id: taskId,
        fileName: t('upload.failedToCancel'),
        status: 'error' as const,
        progress: 0,
        message: t('upload.failedToCancel'),
        timestamp: Date.now(),
      },
    ];
  }
};

// Search state
const showSearch = ref(false);
const searchQuery = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

/**
 * Search functionality
 */
const toggleSearch = () => {
  showSearch.value = !showSearch.value;
  if (showSearch.value) {
    nextTick(() => {
      searchInputRef.value?.focus();
    });
  } else {
    terminal?.focus();
  }
};

const handleSearch = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value, { incremental: true });
  }
};

const handleSearchNext = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value);
  }
};

const handleSearchPrev = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findPrevious(searchQuery.value);
  }
};

const closeSearch = () => {
  showSearch.value = false;
  searchQuery.value = '';
  nextTick(() => {
    terminal?.focus();
  });
};

// Output deduplication tracking
let lastSeq = 0;
let unlistenFn: UnlistenFn | null = null;
let unlistenDisconnect: UnlistenFn | null = null;

/**
 * Write the backend's buffered initial output (welcome banner, MOTD, first
 * prompt) into the terminal, deduping against the live event stream so the
 * same seq is never written twice. Returns true when content was written.
 */
const writeBufferedOutput = async (): Promise<boolean> => {
  if (props.tabType === 'terminal' || !terminal) return false;

  const bufferedOutput = await sessionApi.getBufferedSSHOutput(
    props.sessionId
  );
  if (bufferedOutput.length === 0) return false;

  let wroteAny = false;
  for (const chunk of bufferedOutput) {
    // Dedupe against the live event stream — the output task emits chunks
    // in real time while the connection comes up, so the same seq can
    // arrive both live AND in the buffer. Without this check the welcome
    // banner is written twice.
    if (chunk.seq > lastSeq) {
      terminal.write(chunk.output);
      lastSeq = Math.max(lastSeq, chunk.seq);
      wroteAny = true;
    }
  }

  if (!wroteAny) return false;

  logger.info('Writing buffered SSH output to terminal', {
    chunks: bufferedOutput.length,
  });

  // Scan initial output for home directory (usually shown in first prompt)
  if (!remoteHomeDir.value) {
    const initialBuffer = bufferedOutput
      .map(c => c.output)
      .join('')
      .split('\n');

    for (const line of initialBuffer) {
      // eslint-disable-next-line no-control-regex
      const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '').trim();

      // Look for paths in initial prompts
      const centosMatch = cleanLine.match(/\[.*@.*\s+(.*)\][#$]/);
      const ubuntuMatch = cleanLine.match(/.*@.*:(.*)[#$]/);

      const initialPath = centosMatch?.[1] || ubuntuMatch?.[1];
      if (initialPath && initialPath.startsWith('/')) {
        remoteHomeDir.value = initialPath;
        logger.info('Cached initial remote home directory', {
          home: remoteHomeDir.value,
        });
        break;
      }
    }
  }

  return true;
};

/**
 * Establish connection via session store and API
 */
const connectSession = async (cols: number, rows: number): Promise<void> => {
  if (!props.sessionId) {
    throw new Error('sessionId is required');
  }

  const sessionExists = sessionStore.hasSession(props.sessionId);

  try {
    if (!sessionExists) {
      if (props.tabType === 'terminal') {
        logger.info('Creating local terminal session', {
          sessionId: props.sessionId,
        });
        await sessionStore.createLocalSession(
          props.sessionId,
          props.sessionId,
          cols,
          rows
        );
      } else {
        // Get session info from store (includes serverName)
        const session = sessionStore.getSession(props.sessionId);
        const serverName =
          session?.connectionParams?.serverName || props.ip || 'Unknown';

        // Split panes resolve credentials from the non-reactive cache
        // (pre-seeded by splitActivePane keyed by pane/session id)
        const cached = sessionStore.getCachedCredentials(props.sessionId);

        await sessionStore.createSSHSession(
          props.sessionId,
          props.sessionId,
          serverName,
          props.ip || '',
          props.port || 22,
          props.username || '',
          props.password || cached?.password || '',
          props.privateKeyPath || cached?.privateKeyPath || null,
          props.keyPassphrase || cached?.keyPassphrase || null,
          cols,
          rows
        );
      }
    }

    if (props.tabType !== 'terminal') {
      // The welcome banner / MOTD is buffered by the backend for a short
      // window after connect. Poll the buffer instead of sleeping a fixed
      // amount so we return as soon as content is ready (and never block on a
      // magic timer). `writeBufferedOutput` dedupes against live events.
      const maxAttempts = 14;
      let attempts = 0;
      while (attempts < maxAttempts) {
        if (await writeBufferedOutput()) break;
        attempts += 1;
        await new Promise(resolve => setTimeout(resolve, 200));
      }
    }
  } catch (error) {
    logger.error('Connection failed', error);
    throw error;
  }
};

/**
 * Window resize handler (managed via onActivated/onDeactivated for KeepAlive).
 * Defined at setup top-level so the lifecycle hooks can reference it.
 */
const handleResize = (): void => {
  if (fitAddon) {
    fitAddon.fit();
  }
};

let resizeObserver: ResizeObserver | null = null;

/**
 * Setup SSH output event listener before connection
 */
const setupSSHOutputListener = async (sessionId: string): Promise<void> => {
  if (unlistenFn) {
    await unlistenFn();
  }

  unlistenFn = await listen(`ssh-output-${sessionId}`, (event: {
    payload?: unknown;
  }) => {
    try {
      const payload = event.payload as
        | { seq?: number; output?: string; ts?: number }
        | undefined;

      if (
        payload?.seq !== undefined &&
        payload.output !== undefined &&
        terminal
      ) {
        if (payload.seq > lastSeq) {
          terminal.write(String(payload.output));
          lastSeq = payload.seq;

          // Monitor high latency
          if (payload.ts && Date.now() - payload.ts > LATENCY_THRESHOLD_MS) {
            logger.debug('High latency in SSH output', {
              latency: Date.now() - payload.ts,
            });
          }
        }
      }
    } catch (e) {
      logger.error('Terminal write failed', e);
    }
  });
};

/**
 * Listen for the backend's spontaneous disconnect event.
 *
 * When the SSH connection drops (idle timeout on the server, network blip,
 * NAT dropping the idle TCP connection), the backend I/O task emits
 * `ssh-disconnected-{id}` and stops reading/writing — but the frontend
 * session store still believes the session is connected. Without this
 * listener, keystrokes keep being emitted into a dead channel (silently
 * dropped), which surfaces to the user as "can't type characters" after the
 * terminal has been idle for a while. Listeners must be registered for both
 * SSH and local terminals so the session status is marked accordingly.
 */
const setupDisconnectListener = async (sessionId: string): Promise<void> => {
  if (unlistenDisconnect) {
    await unlistenDisconnect();
    unlistenDisconnect = null;
  }
  if (!sessionId) return;

  unlistenDisconnect = await listen(`ssh-disconnected-${sessionId}`, () => {
    // Only react if this component still owns the session; switching tabs or
    // unmounting unregisters the listener first.
    if (props.sessionId !== sessionId) return;

    logger.warn('SSH session disconnected remotely', { sessionId });

    if (terminal) {
      terminal.write('\r\n\x1b[31m[connection lost]\x1b[0m\r\n');
    }

    // Mark the session as disconnected so the rest of the app (tab badges,
    // session stats) reflects reality, without removing it — the user can
    // still re-open the tab to reconnect.
    sessionStore.updateSessionStatus(sessionId, 'disconnected');
  });
};

/**
 * Connect (or reconnect) to the session identified by `sessionId`.
 * Called on mount and whenever the sessionId prop changes.
 */
const connectToSession = async (sessionId: string): Promise<void> => {
  if (!sessionId) return;
  lastSeq = 0;

  try {
    // Register listener BEFORE connection to catch welcome message
    await setupSSHOutputListener(sessionId);
    await setupDisconnectListener(sessionId);

    if (terminal) {
      // First, ensure we have the correct size before connecting
      if (fitAddon) {
        fitAddon.fit();
      }

      await connectSession(terminal.cols, terminal.rows);

      // Re-sync after a short delay to ensure backend is ready and listener is active
      if (pendingResizeTimer) clearTimeout(pendingResizeTimer);
      pendingResizeTimer = setTimeout(() => {
        if (!terminal) return;
        if (fitAddon && props.sessionId === sessionId) {
          fitAddon.fit();
          emit(`ssh-resize-${sessionId}`, {
            cols: terminal.cols,
            rows: terminal.rows,
          });
        }
      }, 500);
    }
  } catch (error) {
    logger.error('Connection failed', error);
  }
};

// Watch sessionId changes and connect/disconnect accordingly
watch(
  () => props.sessionId,
  (newSessionId, oldSessionId) => {
    if (newSessionId && newSessionId !== oldSessionId) {
      void connectToSession(newSessionId);
    }
  }
);

// Watch for cursor style changes
watch(
  () => settingsStore.terminal.cursorStyle,
  newStyle => {
    if (terminal) {
      terminal.options.cursorStyle = newStyle;
    }
  }
);

// Watch for cursor blink changes
watch(
  () => settingsStore.terminal.cursorBlink,
  newBlink => {
    if (terminal) {
      terminal.options.cursorBlink = newBlink;
    }
  }
);

// Watch for font size changes
watch(
  () => settingsStore.terminal.fontSize,
  newSize => {
    if (terminal) {
      terminal.options.fontSize = newSize;
      fitAddon?.fit();
    }
  }
);

/** Resolve the selected terminal theme & apply it to the live xterm instance. */
function applyTerminalTheme() {
  if (!terminal) return;
  const theme = resolveTerminalTheme(
    settingsStore.terminal.theme,
    themeManager.getActualTheme()
  );
  terminal.options.theme = toXtermTheme(theme);
  // Keep the area around the terminal screen theme-colored. fit() floors the
  // row count, so the screen is a few px shorter than the container; the
  // leftover strip sits on the container background (the xterm viewport is
  // transparent, see CSS) — paint it with the theme background so light
  // themes don't show a black/white bar at the bottom.
  if (terminalRef.value) {
    terminalRef.value.style.backgroundColor = theme.background;
  }
}

// Watch for the terminal theme setting, and re-apply on app light/dark changes
// (relevant when the theme is `system`).
watch(
  () => settingsStore.terminal.theme,
  () => applyTerminalTheme()
);

onMounted(() => {
  window.addEventListener('theme-changed', applyTerminalTheme);
});
onUnmounted(() => {
  window.removeEventListener('theme-changed', applyTerminalTheme);
});

/**
 * Adaptive monitoring refresh rate
 * 700ms when dashboard is open and Performance tab is active, 3s otherwise.
 */
watch(
  [showDashboard, activeDashboardTab, () => props.sessionId],
  async ([show, tab, sid]) => {
    if (!sid) return;
    const interval = show && tab === 'system' ? 700 : 3000;
    try {
      await invoke('set_ssh_status_refresh_rate', {
        sessionId: sid,
        intervalMs: interval,
      });
      logger.debug('Refreshed rate updated', { sid, interval });
    } catch (error) {
      logger.warn('Failed to update refresh rate', error);
    }
  },
  { immediate: true }
);

// Handle activation when switching back to this tab (KeepAlive support)
onActivated(() => {
  nextTick(() => {
    if (fitAddon) {
      fitAddon.fit();
      terminal?.focus();

      // Re-sync terminal dimensions with the backend after activation
      if (terminal && props.sessionId) {
        emit(`ssh-resize-${props.sessionId}`, {
          cols: terminal.cols,
          rows: terminal.rows,
        });
      }
    }
    // Re-establish the metrics listener that was torn down on deactivate.
    void setupStatusListener();
  });
  window.addEventListener('resize', handleResize);
});

// Handle deactivation (KeepAlive) — stop listening to resize, and drop the
// periodic status-metrics listener for this hidden tab so its per-poll chart
// recomputation doesn't keep churning in the background. (Terminal output is
// intentionally left flowing to keep the buffer in sync.)
onDeactivated(() => {
  window.removeEventListener('resize', handleResize);
  if (statusUnlisten) {
    statusUnlisten();
    statusUnlisten = null;
  }
});

/**
 * Cleanup on unmount.
 * With KeepAlive, this only fires when the tab is truly closed or the cache
 * is purged. NOTE: does NOT disconnect the session — session lifecycle is
 * owned by the tab layer.
 */
onUnmounted(async () => {
  window.removeEventListener('resize', handleResize);
  if (pendingResizeTimer) clearTimeout(pendingResizeTimer);
  if (terminalRef.value && handleTerminalContextMenu) {
    terminalRef.value.removeEventListener(
      'contextmenu',
      handleTerminalContextMenu
    );
    handleTerminalContextMenu = null;
  }
  resizeObserver?.disconnect();
  resizeObserver = null;
  if (statusUnlisten) {
    statusUnlisten();
    statusUnlisten = null;
  }
  await cleanupResources();
});

/**
 * Cleanup terminal resources
 *
 * NOTE: does NOT disconnect the session — session lifecycle is owned by the
 * tab layer (closeTab / closePane / cleanupAllSessions). Disconnecting here
 * would kill the session whenever this component unmounts for layout reasons
 * (e.g. a split re-renders the single-pane branch into a SplitRenderer tree,
 * unmounting the original pane's component).
 */
const cleanupResources = async (): Promise<void> => {
  if (disposeIMEFix) {
    disposeIMEFix();
    disposeIMEFix = null;
  }

  terminal?.dispose();

  if (unlistenFn) {
    try {
      await unlistenFn();
      unlistenFn = null;
    } catch (e) {
      logger.error('Event unlisten failed', e);
    }
  }

  if (unlistenDisconnect) {
    try {
      await unlistenDisconnect();
      unlistenDisconnect = null;
    } catch (e) {
      logger.error('Disconnect event unlisten failed', e);
    }
  }

  if (unlistenDrag) await unlistenDrag();
  if (unlistenDragEnter) await unlistenDragEnter();
  if (unlistenDragLeave) await unlistenDragLeave();
  if (unlistenUpload) await unlistenUpload();
  if (unlistenDownload) await unlistenDownload();
};

/**
 * Expose cleanup method to parent
 */
defineExpose({
  cleanupResources,
});

/**
 * Component lifecycle
 *
 * IMPORTANT: all watch/onActivated/onDeactivated/onUnmounted hooks are
 * registered at setup top-level (above). Registering them inside this async
 * function after `await` would silently no-op — Vue has no active instance
 * in async continuations, so the cleanup hooks would never run.
 */
onMounted(() => {
  void initialize();
});

const initialize = async (): Promise<void> => {
  logger.info('Terminal component mounted', { sessionId: props.sessionId });
  await setupStatusListener();

  // Listen for session-specific upload progress
  unlistenUpload = await listen<UploadProgressPayload>(
    `ssh-upload-progress-${props.sessionId}`,
    event => {
      const payload = event.payload;
      // Only update if it belongs to this session
      if (payload.sessionId === props.sessionId) {
        updateUploadTask(payload.taskId, {
          status: payload.status,
          progress: Math.floor(payload.progress),
          message: payload.message,
          uploadedBytes: payload.uploadedBytes,
          fileSize: payload.totalBytes,
          speed: payload.speed,
          error: payload.error || undefined,
          eta:
            payload.speed > 0
              ? (payload.totalBytes - payload.uploadedBytes) / payload.speed
              : undefined,
        });
      }
    }
  );

  // Listen for session-specific download progress
  unlistenDownload = await listen<UploadProgressPayload>(
    `ssh-download-progress-${props.sessionId}`,
    event => {
      const payload = event.payload;
      if (payload.sessionId === props.sessionId) {
        // Map the backend "downloading" status to the transfer queue, and
        // retain the original direction on the queued task so the cancel
        // routing and the queue rendering stay correct.
        const nextStatus =
          payload.status === 'success' ||
          payload.status === 'error' ||
          payload.status === 'cancelled' ||
          payload.status === 'downloading'
            ? payload.status
            : 'downloading';
        updateUploadTask(payload.taskId, {
          direction: 'download',
          status: nextStatus as UploadTask['status'],
          progress: Math.floor(payload.progress),
          message: payload.message,
          uploadedBytes: payload.uploadedBytes,
          fileSize: payload.totalBytes,
          speed: payload.speed,
          error: payload.error || undefined,
          eta:
            payload.speed > 0
              ? (payload.totalBytes - payload.uploadedBytes) / payload.speed
              : undefined,
        });
      }
    }
  );

  // Listen for Tauri's native drag-drop event to get absolute paths
  unlistenDrag = await listen<{ paths: string[] }>(
    'tauri://drag-drop',
    event => {
      // Only handle if drag-drop occurs within the terminal area or window
      handleFileDrop(event.payload.paths);
      isDragging.value = false;
    }
  );

  unlistenDragEnter = await listen('tauri://drag-enter', () => {
    isDragging.value = true;
  });

  unlistenDragLeave = await listen('tauri://drag-leave', () => {
    isDragging.value = false;
  });

  if (!terminalRef.value) return;

  // Initialize xterm.js terminal
  terminal = new Terminal({
    scrollback: settingsStore.terminal.scrollback,
    fontSize: settingsStore.terminal.fontSize,
    fontFamily: settingsStore.terminal.fontFamily,
    rows: 24,
    cols: 80,
    cursorBlink: settingsStore.terminal.cursorBlink,
    cursorStyle: settingsStore.terminal.cursorStyle,
    theme: toXtermTheme(
      resolveTerminalTheme(
        settingsStore.terminal.theme,
        themeManager.getActualTheme()
      )
    ),
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  searchAddon = new SearchAddon();
  terminal.loadAddon(searchAddon);

  // Use WebGL renderer for better performance
  try {
    terminal.loadAddon(new WebglAddon());
  } catch {
    logger.warn('WebGL addon unavailable, using standard renderer');
  }

  terminal.open(terminalRef.value);

  // Paint the container background with the terminal theme color (keeps the
  // strip below the fitted screen invisible in any theme).
  applyTerminalTheme();

  disposeIMEFix = attachMacWebKitIMESymbolFix(terminal);

  // Register OSC 7 handler for current working directory detection
  // OSC 7 ; file://hostname/path ST
  terminal.parser.registerOscHandler(7, data => {
    try {
      if (data.includes('://')) {
        const urlStr = data.includes('://') ? data : `file://${data}`;
        const url = new window.URL(urlStr);
        currentRemotePath.value = decodeURIComponent(url.pathname);
      } else {
        currentRemotePath.value = data;
      }
      if (currentRemotePath.value.startsWith('/')) {
        lastKnownAbsolutePath.value = normalizeRemotePath(
          currentRemotePath.value
        );
        hasOscPath.value = true;
      }
      logger.debug('Detected remote CWD (OSC 7)', {
        path: currentRemotePath.value,
      });
    } catch {
      currentRemotePath.value = data;
    }
    return true;
  });

  // Register OSC 9;9 handler (another CWD sequence)
  terminal.parser.registerOscHandler(9, data => {
    if (data.startsWith('9;')) {
      const path = data.substring(2);
      if (path) {
        currentRemotePath.value = path;
        if (currentRemotePath.value.startsWith('/')) {
          lastKnownAbsolutePath.value = normalizeRemotePath(
            currentRemotePath.value
          );
          hasOscPath.value = true;
        }
        logger.debug('Detected remote CWD (OSC 9;9)', { path });
      }
      return true;
    }
    return false;
  });

  // Also monitor terminal title as some shells put CWD there
  terminal.onTitleChange(title => {
    if (!title) return;

    // Pattern 1: user@host: /path
    if (title.includes(': ')) {
      const parts = title.split(': ');
      const potentialPath = parts[parts.length - 1].trim();
      if (potentialPath.startsWith('/') || potentialPath.startsWith('~/')) {
        if (
          currentRemotePath.value === '.' ||
          currentRemotePath.value === '~'
        ) {
          currentRemotePath.value = potentialPath;
          if (currentRemotePath.value.startsWith('/')) {
            lastKnownAbsolutePath.value = normalizeRemotePath(
              currentRemotePath.value
            );
          }
        }
      }
    }
    // Pattern 2: [user@host path]
    else if (title.includes('[') && title.includes(']')) {
      const match = title.match(/\[.*@.*\s+(.*)\]/);
      if (match && match[1]) {
        if (
          currentRemotePath.value === '.' ||
          currentRemotePath.value === '~'
        ) {
          currentRemotePath.value = match[1].trim();
          if (currentRemotePath.value.startsWith('/')) {
            lastKnownAbsolutePath.value = normalizeRemotePath(
              currentRemotePath.value
            );
          }
        }
      }
    }
  });

  // Handle right-click for direct paste or copy-on-selection
  handleTerminalContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    if (terminal?.hasSelection()) {
      const selection = terminal.getSelection();
      if (selection) {
        navigator.clipboard.writeText(selection).catch(err => {
          logger.error('Copy selection failed', err);
        });
        terminal.clearSelection();
      }
    } else {
      try {
        navigator.clipboard
          .readText()
          .then(text => {
            if (text && terminal) {
              terminal.paste(text);
            }
          })
          .catch(err => {
            logger.error('Right-click paste failed', err);
          });
      } catch (err) {
        logger.error('Right-click paste failed', err);
      }
    }
  };
  terminalRef.value?.addEventListener('contextmenu', handleTerminalContextMenu);

  // Handle keyboard shortcuts and allow global app shortcuts to bubble up
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    if (event.type !== 'keydown') return true;

    const isMac = navigator.userAgent.includes('Mac');
    const isControlKey = isMac ? event.metaKey : event.ctrlKey;
    const key = event.key.toLowerCase();

    // 1. Allow global app shortcuts to bubble up to the window
    // This includes Cmd+T, Cmd+Shift+T, Cmd+W, Cmd+P, Cmd+, and Cmd+Shift+P
    if (
      isControlKey &&
      (['t', 'w', 'p', ','].includes(key) || event.shiftKey)
    ) {
      return false;
    }

    // 1b. Cmd+D (Mac) / Ctrl+Shift+D (Win) for split pane — let it bubble
    // Pure Ctrl+D (no meta, no shift) must pass through to shell as EOF
    if (key === 'd') {
      if (isMac && event.metaKey) {
        return false;
      }
      if (!isMac && event.ctrlKey && event.shiftKey) {
        return false;
      }
    }

    // 2. Handle clipboard shortcuts (Cmd+C/V on Mac, Ctrl+C/V on Windows/Linux)
    // We let KeyC and KeyV bubble to the browser so it can handle native Copy/Paste
    // xterm.js manages a hidden textarea that receives these events correctly.
    // This avoids the "Permission required" popup from navigator.clipboard.readText().
    if (isControlKey && (event.code === 'KeyC' || event.code === 'KeyV')) {
      // For Ctrl+C on Windows/Linux, if there is a selection, we copy and don't send to terminal.
      if (!isMac && event.code === 'KeyC' && terminal?.hasSelection()) {
        const selection = terminal.getSelection();
        if (selection) {
          navigator.clipboard.writeText(selection);
        }
        return false;
      }
      return true; // Let browser/xterm handle it natively
    }

    // 3. Other internal shortcuts
    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyA') {
      terminal?.selectAll();
      return false;
    }

    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyF') {
      toggleSearch();
      return false;
    }

    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyK') {
      terminal?.clear();
      return false;
    }

    return true;
  });

  // Use ResizeObserver for robust layout management
  resizeObserver = new ResizeObserver(() => {
    if (fitAddon) {
      try {
        fitAddon.fit();
      } catch (e) {
        logger.error('Fit error', e);
      }
    }
  });

  if (terminalRef.value) {
    resizeObserver.observe(terminalRef.value);
  }

  await nextTick();
  fitAddon.fit();
  terminal.focus();

  // Handle terminal resize and notify backend
  terminal.onResize(({ cols, rows }) => {
    if (props.sessionId) {
      emit(`ssh-resize-${props.sessionId}`, { cols, rows });
    }
  });

  /**
   * Handle terminal input
   * Using onData instead of onKey to properly handle IME (Chinese input)
   */
  terminal.onData(async (data: string) => {
    const session = props.sessionId
      ? sessionStore.getSession(props.sessionId)
      : undefined;
    const hasSession = !!session;
    const isDead =
      session && (session.status === 'disconnected' || session.status === 'error');

    if (hasSession && !isDead) {
      // Send data immediately to ensure interactive tools (vim, ssh, etc.)
      // work correctly without broken escape sequences or latency.
      try {
        await emit(`ssh-input-${props.sessionId}`, { input: data });
      } catch (error) {
        logger.error('Input emit failed', error);
      }
    } else if (data.length === 1 && data >= ' ' && data !== '\x7f') {
      // No active session: echo printable characters locally
      terminal?.write(data);
    }
  });

  // Initial connection is handled when a valid `sessionId` is provided
  await connectToSession(props.sessionId);
};
</script>

<template>
  <div class="remote-connection-view">
    <ServerDashboard
      :show="showDashboard"
      :active-tab="activeDashboardTab"
      :session-id="props.sessionId"
      :history="statusHistory"
      :upload-tasks="uploadTasks"
      @clear-tasks="clearUploadTasks"
      @toggle="showDashboard = !showDashboard"
      @update:active-tab="activeDashboardTab = $event"
      @pause-task="pauseUploadTask"
      @resume-task="resumeUploadTask"
      @cancel-task="cancelUploadTask"
    />
    <div ref="terminalRef" class="terminal-container" />

    <!-- Open the standalone file manager for this session -->
    <button
      type="button"
      class="open-files-btn"
      :title="t('dashboard.openInWindow')"
      @click="openFileManager"
    >
      <FolderTree :size="14" />
    </button>

    <!-- Open the port-forwarding manager for this session -->
    <button
      type="button"
      class="open-tunnels-btn"
      :title="t('tunnel.openPanel')"
      @click="showTunnelPanel = true"
    >
      <Waypoints :size="14" />
    </button>

    <TunnelManagerPanel
      :session-id="props.sessionId"
      :visible="showTunnelPanel"
      @update:visible="showTunnelPanel = $event"
    />

    <!-- Drag and Drop Overlay -->
    <div v-if="isDragging" class="drag-drop-overlay">
      <div class="overlay-content">
        <span class="icon">📂</span>
        <p>{{ t('ssh.dropFilesHere') }}</p>
        <div class="target-path-display">
          <div class="path-label">{{ t('ssh.targetDirectory') }}</div>
          <input
            v-model="currentRemotePath"
            class="drop-path-input"
            :placeholder="t('ssh.autoDetectedPath')"
            @click.stop
            @mousedown.stop
          />
          <div v-if="!currentRemotePath" class="path-status warning">
            ⚠️ {{ t('ssh.noPathDetected') }}
          </div>
          <div
            v-else-if="currentRemotePath.startsWith('/')"
            class="path-status success"
          >
            ✓ {{ t('ssh.absolutePath') }}
          </div>
          <div v-else class="path-status info">
            ℹ️ {{ t('ssh.relativePath') }}
          </div>
        </div>
        <div class="overlay-tip">
          💡 {{ t('ssh.editPathBeforeDrop') }}
        </div>
      </div>
    </div>

    <!-- Pre-upload Confirmation Dialog -->
    <div v-if="uploadConfirm.open" class="upload-confirm-overlay">
      <div class="upload-confirm-card" @click.stop>
        <div class="upload-confirm-header">
          <h3>{{ t('upload.confirmTitle') }}</h3>
          <button
            type="button"
            class="upload-confirm-close"
            aria-label="Close"
            @click="closeUploadConfirm"
          >
            ×
          </button>
        </div>

        <div class="upload-confirm-body">
          <div class="confirm-dir-group">
            <label class="confirm-label">{{
              t('upload.targetDirectory')
            }}</label>
            <input
              v-model="uploadConfirm.targetDir"
              class="confirm-dir-input"
              type="text"
              spellcheck="false"
              @input="recomputeConfirmTargets"
              @keydown.enter.exact.stop.prevent="confirmUploads"
            />
          </div>

          <div class="confirm-files">
            <div
              v-for="file in uploadConfirm.files"
              :key="file.localPath"
              class="confirm-file-row"
            >
              <div class="confirm-file-info">
                <span class="confirm-file-name">{{ file.fileName }}</span>
                <span class="confirm-file-path" :title="file.remotePath">
                  {{ t('upload.to') }}: {{ file.remotePath }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div class="upload-confirm-footer">
          <button
            type="button"
            class="btn-secondary"
            @click="closeUploadConfirm"
          >
            {{ t('upload.cancelUpload') }}
          </button>
          <button
            type="button"
            class="btn-primary"
            :disabled="confirmCount === 0"
            @click="confirmUploads"
          >
            {{ t('upload.confirmUpload', { count: confirmCount }) }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="showSearch" class="terminal-search-box">
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        type="text"
        :placeholder="t('ssh.searchPlaceholder')"
        @input="handleSearch"
        @keydown.enter.exact.stop.prevent="handleSearchNext"
        @keydown.shift.enter.stop.prevent="handleSearchPrev"
        @keydown.escape.stop.prevent="closeSearch"
      />
      <div class="search-actions">
        <button @click.stop="handleSearchPrev">↑</button>
        <button @click.stop="handleSearchNext">↓</button>
        <button @click.stop="closeSearch">×</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.remote-connection-view {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: transparent;
  padding: 0;
  box-sizing: border-box;
  position: relative;
}

/* Floating "open standalone file manager" button (bottom-right, low-key). */
.open-files-btn {
  position: absolute;
  right: 18px;
  bottom: 18px;
  z-index: 90;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--color-border-secondary, #333);
  border-radius: 8px;
  background: rgba(45, 45, 45, 0.85);
  color: var(--color-text-secondary, #9d9d9d);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.open-files-btn:hover {
  background: rgba(60, 60, 60, 0.9);
  color: var(--color-text-primary, #fff);
  border-color: var(--color-border-primary, #555);
}

/* Floating "manage port forwarding" button (bottom-right, low-key), sits to
   the left of the file-manager button. */
.open-tunnels-btn {
  position: absolute;
  right: 54px;
  bottom: 18px;
  z-index: 90;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--color-border-secondary, #333);
  border-radius: 8px;
  background: rgba(45, 45, 45, 0.85);
  color: var(--color-text-secondary, #9d9d9d);
  cursor: pointer;
  transition:
    background 0.15s,
    color 0.15s,
    border-color 0.15s;
}
.open-tunnels-btn:hover {
  background: rgba(60, 60, 60, 0.9);
  color: var(--color-text-primary, #fff);
  border-color: var(--color-border-primary, #555);
}

.terminal-search-box {
  position: absolute;
  top: 10px;
  right: 20px;
  z-index: 100;
  display: flex;
  align-items: center;
  background-color: #2d2d2d;
  padding: 4px 10px;
  border-radius: 4px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  border: 1px solid #444;
  transition: border-color 0.2s;
}

.terminal-search-box:focus-within {
  border-color: #facc15;
}

.terminal-search-box input {
  background: transparent;
  border: none;
  color: #fff;
  outline: none;
  font-size: 13px;
  padding: 4px;
  width: 200px;
}

.drag-drop-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(30, 30, 30, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  pointer-events: none; /* 让事件穿透到 Tauri 的系统监听 */
  border: 2px dashed #facc15;
  box-sizing: border-box;
}

.overlay-content {
  text-align: center;
  color: #facc15;
}

.overlay-content .icon {
  font-size: 48px;
  display: block;
  margin-bottom: 10px;
}

.overlay-content p {
  font-size: 16px;
  font-weight: 500;
  margin-bottom: 8px;
}

.target-path-display {
  background: rgba(0, 0, 0, 0.4);
  padding: 12px 16px;
  border-radius: 6px;
  margin-bottom: 12px;
  border: 1px solid rgba(250, 204, 21, 0.2);
}

.path-label {
  font-size: 12px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 6px;
}

.drop-path-input {
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  padding: 6px 12px;
  color: #facc15;
  font-family: var(--font-mono);
  font-size: 13px;
  width: 288px;
  text-align: center;
  outline: none;
  pointer-events: auto;
}

.drop-path-input:focus {
  border-color: #facc15;
}

.path-status {
  font-size: 12px;
  margin-top: 6px;
  padding: 6px 8px;
  border-radius: 3px;
  font-family: var(--font-mono);
}

.path-status.success {
  color: #4ade80;
  background: rgba(74, 222, 128, 0.1);
}

.path-status.warning {
  color: #facc15;
  background: rgba(250, 204, 21, 0.1);
}

.path-status.info {
  color: #60a5fa;
  background: rgba(96, 165, 250, 0.1);
}

.overlay-tip {
  font-size: 12px;
  color: #888;
}
.search-actions {
  display: flex;
  gap: 4px;
  margin-left: 8px;
}

.search-actions button {
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  padding: 4px 6px;
  font-size: 14px;
  line-height: 1;
  border-radius: 3px;
  transition: all 0.2s;
}

.search-actions button:hover {
  color: #fff;
  background-color: #444;
}

.terminal-container {
  width: 100%;
  height: 100%;
  transition: margin-right 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

/*
 * Make the xterm element fill the whole container. fit() floors the row
 * count, so the terminal screen is usually a few px shorter than the
 * container; with height:100% the .xterm-viewport covers the whole area.
 *
 * The viewport's default background is hardcoded #000 in xterm.css, which
 * shows as a black bar below the screen in light themes — make it
 * transparent so the container background (painted with the terminal theme
 * background by applyTerminalTheme) shows through instead.
 *
 * The padding insets the terminal screen from the pane edges so the first
 * character of a row doesn't hug the border. It must live on the .xterm
 * element (not the container): FitAddon subtracts the terminal element's own
 * padding when computing cols/rows, so the screen always fits inside the
 * padding — padding on the measured container would be ignored and the last
 * row could overflow past the bottom edge.
 */
.terminal-container :deep(.xterm) {
  height: 100%;
  padding: 8px;
  box-sizing: border-box;
}
.terminal-container :deep(.xterm-viewport) {
  background-color: transparent;
}

/* Pre-upload confirmation dialog */
.upload-confirm-overlay {
  position: fixed;
  inset: 0;
  background-color: var(--color-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  padding: 16px;
}

.upload-confirm-card {
  width: 460px;
  max-width: 100%;
  max-height: 80vh;
  background-color: var(--color-bg-elevated);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.upload-confirm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--color-border-secondary);
}

.upload-confirm-header h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.upload-confirm-close {
  background: none;
  border: none;
  color: var(--color-text-tertiary);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  transition: color var(--transition-fast), background-color var(--transition-fast);
}

.upload-confirm-close:hover {
  color: var(--color-text-primary);
  background-color: var(--color-interactive-hover);
}

.upload-confirm-body {
  padding: 14px 16px;
  overflow-y: auto;
}

.confirm-dir-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.confirm-label {
  font-size: 12px;
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.confirm-dir-input {
  background-color: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  color: var(--color-text-primary);
  padding: 8px 10px;
  font-size: 13px;
  font-family: var(--font-mono);
}

.confirm-dir-input:focus {
  outline: none;
  border-color: var(--color-accent);
  box-shadow: var(--focus-ring);
}

.confirm-files {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.confirm-file-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  background-color: var(--color-interactive-hover);
}

.confirm-file-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.confirm-file-name {
  font-size: 13px;
  color: var(--color-text-primary);
  font-weight: 500;
  word-break: break-all;
}

.confirm-file-path {
  font-size: 12px;
  color: var(--color-text-tertiary);
  font-family: var(--font-mono);
  word-break: break-all;
}

.upload-confirm-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 16px;
  border-top: 1px solid var(--color-border-secondary);
}

.upload-confirm-footer .btn-secondary,
.upload-confirm-footer .btn-primary {
  padding: 8px 16px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background-color var(--transition-fast), color var(--transition-fast);
}

.upload-confirm-footer .btn-secondary {
  background-color: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  border-color: var(--color-border-primary);
}

.upload-confirm-footer .btn-secondary:hover {
  background-color: var(--color-bg-elevated);
}

.upload-confirm-footer .btn-primary {
  background-color: var(--color-accent);
  color: var(--color-bg-primary);
  border-color: var(--color-accent);
}

.upload-confirm-footer .btn-primary:hover {
  background-color: color-mix(in srgb, var(--color-accent) 85%, white);
}

.upload-confirm-footer .btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
