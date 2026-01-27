<script setup lang="ts">
import { onMounted, onUnmounted, ref, nextTick, watch, onActivated } from 'vue';
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
import ServerDashboard from './ServerDashboard.vue';

const logger = createLogger('REMOTE_CONNECTION_VIEW');

const sessionStore = useSessionStore();
const settingsStore = useSettingsStore();

const showDashboard = ref(false);
const activeDashboardTab = ref<'system' | 'uploads' | null>('system');

interface ServerStatus {
  cpuUsage: number;
  memUsage: number;
  memTotal: number;
  memUsed: number;
  memAvail: number;
  swapUsage: number;
  swapTotal: number;
  swapUsed: number;
  diskUsage: number;
  diskTotal: number;
  diskUsed: number;
  diskAvail: number;
  netUp: number;
  netDown: number;
  latency: number;
  loadAvg: [number, number, number];
  uptime: string;
}

const statusHistory = ref<ServerStatus[]>([]);
const MAX_HISTORY = 60;
let statusUnlisten: UnlistenFn | null = null;

const setupStatusListener = async () => {
  if (statusUnlisten) statusUnlisten();
  statusHistory.value = []; // Clear history for new session
  if (!props.sessionId) return;

  statusUnlisten = await listen<ServerStatus>(
    `ssh-status-${props.sessionId}`,
    event => {
      statusHistory.value.push(event.payload);
      if (statusHistory.value.length > MAX_HISTORY) {
        statusHistory.value.shift();
      }
    }
  );
};

// Terminal configuration constants - Now acting as defaults or base
const TERMINAL_CONFIG = {
  THEME: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    selectionBackground: '#facc15', // Bright yellow background for selection
    selectionForeground: '#000000', // Black text for selected content
  },
};

const INPUT_BUFFER_CONFIG = {
  FLUSH_THRESHOLD: 32,
  FLUSH_DELAY_MS: 20,
};

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
}

const props = withDefaults(defineProps<Props>(), {
  sessionId: '',
  tabType: 'ssh',
  ip: '',
  port: 22,
  username: '',
  password: '',
});

const terminalRef = ref<HTMLElement>();
const isDragging = ref(false);
const currentRemotePath = ref('.');
const remoteHomeDir = ref('');
const lastPathDetectionSource = ref<string>('none'); // Track how we detected the path
const lastKnownAbsolutePath = ref('');
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;

// Upload task tracking
interface UploadTask {
  id: string;
  fileName: string;
  remotePath?: string;
  status: 'pending' | 'uploading' | 'success' | 'error';
  progress: number;
  message: string;
  timestamp: number;
  error?: string;
  // Speed tracking
  fileSize?: number;
  uploadedBytes?: number;
  startTime?: number;
  speed?: number; // bytes per second
  eta?: number; // seconds remaining
}

const uploadTasks = ref<UploadTask[]>([]);

const addUploadTask = (fileName: string): string => {
  const id = `upload-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  uploadTasks.value.unshift({
    id,
    fileName,
    status: 'pending',
    progress: 0,
    message: 'Preparing...',
    timestamp: Date.now(),
  });
  return id;
};

const updateUploadTask = (id: string, updates: Partial<UploadTask>) => {
  const task = uploadTasks.value.find(t => t.id === id);
  if (task) {
    Object.assign(task, updates);
  }
};

const clearUploadTasks = () => {
  // Only clear completed and error tasks, keep uploading/pending tasks
  uploadTasks.value = uploadTasks.value.filter(
    task => task.status === 'uploading' || task.status === 'pending'
  );
};

// Drag and drop listeners
let unlistenDrag: UnlistenFn | null = null;
let unlistenDragEnter: UnlistenFn | null = null;
let unlistenDragLeave: UnlistenFn | null = null;
let unlistenUpload: UnlistenFn | null = null;

interface UploadProgressPayload {
  taskId: string;
  sessionId: string;
  progress: number;
  uploadedBytes: number;
  totalBytes: number;
  status: 'uploading' | 'success' | 'error';
  message: string;
  speed: number;
  error?: string;
}

const normalizeRemotePath = (path: string): string => {
  if (!path.startsWith('/')) return path;
  const parts = path.split('/');
  const stack: string[] = [];
  for (const part of parts) {
    if (!part || part === '.') continue;
    if (part === '..') {
      stack.pop();
    } else {
      stack.push(part);
    }
  }
  return '/' + stack.join('/');
};

const resolveRelativeCd = (
  relativePath: string,
  basePath: string,
  homePath: string
): string => {
  if (!relativePath) return '';
  if (relativePath === '-') return '';
  if (relativePath === '.') return basePath || '';
  if (relativePath === '..') {
    return basePath ? normalizeRemotePath(`${basePath}/..`) : '';
  }
  if (relativePath === '~') {
    return homePath ? normalizeRemotePath(homePath) : '';
  }
  if (relativePath.startsWith('~/')) {
    return homePath
      ? normalizeRemotePath(`${homePath}/${relativePath.slice(2)}`)
      : '';
  }
  if (!basePath) return '';
  return normalizeRemotePath(`${basePath}/${relativePath}`);
};

/**
 * Detect current path from terminal buffer
 */
const detectRemotePath = async () => {
  // Reset path for fresh detection on each drag
  let detectedPath = '';
  let detectionSource = 'none';
  let hasRecentRelativeCd = false; // Track if user did cd .., cd -, etc
  let promptGuess = '';
  let cdGuess = '';
  let relativeCdGuess = '';
  let pwdOutputGuess = '';

  // Try to guess path from terminal buffer (most accurate for interactive shell)
  if (terminal) {
    const buffer = terminal.buffer.active;
    if (!buffer) {
      logger.warn('Terminal buffer not available');
    } else {
      logger.debug('Terminal buffer available', {
        bufferLength: buffer.length,
        cursorX: buffer.cursorX,
        cursorY: buffer.cursorY,
        baseY: buffer.baseY,
      });

      // 1. Scan back from current cursor position for prompts, cd commands, and pwd output
      const maxScanLines = 500; // Increased: Scan deeper to catch recent history (up to 500 lines)
      const startLine = buffer.baseY + buffer.cursorY;
      const endLine = Math.max(0, startLine - maxScanLines);

      // Build a map of lines to avoid scanning too many empty lines
      const lines: Array<{ index: number; text: string }> = [];
      for (let i = startLine; i >= endLine; i--) {
        const line = buffer.getLine(i)?.translateToString(true).trim();
        if (line) {
          lines.push({ index: i, text: line });
        }
      }

      logger.debug('Buffer scan started', {
        cursorY: buffer.cursorY,
        baseY: buffer.baseY,
        startLine,
        endLine,
        totalLines: buffer.length,
        nonEmptyLines: lines.length,
      });

      // Process lines in reverse chronological order (most recent first)
      for (const { index, text } of lines) {
        const line = text;
        // eslint-disable-next-line no-control-regex
        const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '').trim();

        // Check for relative cd commands (these invalidate old absolute paths)
        // Also track cd history for intelligent directory inference
        if (line.includes('cd ')) {
          const relativeCdMatch = line.match(/cd\s+(\.\.|[-]|\.)/);
          const absoluteCdMatch = line.match(/cd\s+(['"]?)([^\s'"]+)\1/);

          if (relativeCdMatch) {
            if (!hasRecentRelativeCd) {
              hasRecentRelativeCd = true;
              logger.debug('Detected relative cd command', {
                command: relativeCdMatch[0],
                lineIndex: index,
              });
            }
            if (!relativeCdGuess) {
              relativeCdGuess = relativeCdMatch[1];
            }
          } else if (absoluteCdMatch && absoluteCdMatch[2]) {
            let path = absoluteCdMatch[2];
            if (path.endsWith('/') && path !== '/') {
              path = path.slice(0, -1);
            }
            if (path.startsWith('/')) {
              logger.debug('Recorded cd history', { path, index });
            } else {
              if (!hasRecentRelativeCd) {
                hasRecentRelativeCd = true;
                logger.debug('Detected relative cd command', {
                  command: `cd ${path}`,
                  lineIndex: index,
                });
              }
              if (!relativeCdGuess) {
                relativeCdGuess = path;
              }
            }
          }
        }

        // Pattern A: Match pwd output - absolute path on a line by itself
        if (!pwdOutputGuess && line.startsWith('/')) {
          // pwd output should:
          // 1. Start with /
          // 2. Contain at least one more /
          // 3. NOT contain spaces (commands can have output with spaces)
          // 4. NOT contain [ or (
          // 5. NOT contain @
          if (
            cleanLine.startsWith('/') &&
            cleanLine.includes('/') &&
            !cleanLine.includes(' ') &&
            !cleanLine.includes('[') &&
            !cleanLine.includes('(') &&
            !cleanLine.includes('@')
          ) {
            pwdOutputGuess = cleanLine;
            logger.debug('Found pwd output', {
              line: cleanLine,
              lineIndex: index,
            });
          }
        }

        // Pattern B: Match Prompt to get terminal "hint"
        // Most recent prompt is most important
        if (!promptGuess) {
          // CentOS style: [root@host /current/path]#
          const centosMatch = line.match(/\[.*@.*\s+(.*)\][#$]/);
          // Ubuntu style: user@host:/path$
          const ubuntuMatch = line.match(/.*@.*:(.*)[#$]/);
          // Zsh style: user@host path %
          const zshMatch = line.match(/.*@.*\s+([^ ]+)\s+%/);

          const hint = centosMatch?.[1] || ubuntuMatch?.[1] || zshMatch?.[1];
          if (
            line.includes('@') &&
            (line.includes('#') || line.includes('$'))
          ) {
            logger.debug('Prompt candidates found', {
              line: cleanLine,
              centosMatch: centosMatch?.[1],
              ubuntuMatch: ubuntuMatch?.[1],
              zshMatch: zshMatch?.[1],
              hint,
            });
          }
          // Accept paths: /, /path, or relative names
          // Reject: ~ or . (as they are generic)
          if (hint && hint !== '~' && hint !== '.') {
            promptGuess = hint.trim();
            logger.debug('Found prompt hint', {
              hint: promptGuess,
              lineIndex: index,
            });
          }
        }

        // Pattern C: Match recent cd commands (including paths with trailing slashes)
        if (!cdGuess && line.includes('cd ')) {
          // Match: cd /path, cd "/path", cd '/path', etc (with optional trailing /)
          const cdMatch = line.match(/cd\s+(['"]?)([^\s'"]+)\1/);
          logger.debug('cd command scan', {
            line: cleanLine,
            hascdMatch: !!cdMatch,
            cdMatch: cdMatch ? [cdMatch[1], cdMatch[2]] : null,
          });
          if (cdMatch && cdMatch[2]) {
            let path = cdMatch[2].trim();
            // Remove trailing slash for consistency
            if (path.endsWith('/') && path !== '/') {
              path = path.slice(0, -1);
            }
            if (path.startsWith('/')) {
              cdGuess = path;
              logger.debug('Found cd command', {
                path: cdGuess,
                lineIndex: index,
              });
            }
          }
        }

        // Early exit if we found everything
        if (pwdOutputGuess && promptGuess && cdGuess) break;
      }

      // 2. Intelligent path resolution with strict priority
      // KEY INSIGHT: The most recent prompt is ALWAYS more reliable than historical pwd output
      // because it reflects the current state of the shell
      logger.debug('Path candidates', { pwdOutputGuess, promptGuess, cdGuess });

      // Intelligent priority logic:
      // 1. Absolute paths are always preferred over relative
      // 2. Most recent data (prompt) is preferred over historical (pwd output)
      // 3. Never use relative paths from Prompt alone - they're unreliable after cd .. or cd -

      if (promptGuess && promptGuess.startsWith('/')) {
        // Highest priority: Prompt shows absolute path directly
        detectedPath = promptGuess;
        detectionSource = 'prompt-absolute';
      } else if (pwdOutputGuess && pwdOutputGuess.startsWith('/')) {
        // Second: pwd output (most reliable source of truth)
        detectedPath = pwdOutputGuess;
        detectionSource = 'pwd-output';
      } else if (cdGuess && cdGuess.startsWith('/')) {
        // Third: cd command with absolute path
        // But verify it matches the current prompt
        const cdLastComponent = cdGuess.split('/').pop();
        if (promptGuess && cdLastComponent === promptGuess) {
          // ✅ MATCH: Prompt confirms this is the right path (e.g., cd /a/b + prompt "b")
          detectedPath = cdGuess;
          detectionSource = 'cd-command-verified';
          logger.debug('✅ cd-command-verified: cd path matches prompt', {
            cdPath: cdGuess,
            cdLastComponent,
            currentPrompt: promptGuess,
          });
        } else if (promptGuess && !promptGuess.startsWith('/')) {
          // ⚠️ CHECK: Prompt is relative but might still match cd
          // Example: cd /usr/local/sankuai executed, now prompt shows "sankuai"
          // This is VALID - prompt shows dir name that matches cd's last component
          if (cdLastComponent === promptGuess) {
            // They DO match! Use the cd path
            detectedPath = cdGuess;
            detectionSource = 'cd-command-verified-from-relative-prompt';
            logger.debug('✅ cd path matches relative prompt', {
              cdPath: cdGuess,
              cdLastComponent,
              currentPrompt: promptGuess,
            });
          } else {
            // ❌ MISMATCH: cd last component != prompt
            // This means user likely did cd .. or some relative operation
            // Example: cd /usr/local/sankuai was executed, but now prompt shows "local"
            // This means cd .. happened - we MUST handle this case specially
            hasRecentRelativeCd = true;
            logger.debug('❌ Mismatch detected: cd != prompt', {
              cdPath: cdGuess,
              cdLastComponent,
              currentPrompt: promptGuess,
              reason: 'Likely relative cd operation (cd .., cd -, etc)',
            });
            // Don't try to reconstruct path here - let the later inference logic handle it
            // It has better context to determine if this is cd .., cd -, etc.
          }
        } else {
          // Use cd path as-is if there's no conflicting prompt
          detectedPath = cdGuess;
          detectionSource = 'cd-command';
        }
      }
      // NOTE: We do NOT use promptGuess as a fallback if it's not absolute
      // Relative prompts (e.g., "local" or "sankuai") after cd .. are unreliable
      // Better to use probed path or home directory as fallback
    }
  }

  logger.info('Buffer scan complete', {
    detectedPath,
    detectionSource,
    hasRecentRelativeCd,
    allCandidates: {
      promptGuess,
      pwdOutputGuess,
      cdGuess,
    },
  });

  // CRITICAL IMPROVEMENT: If we have a relative cd but detectedPath is empty,
  // try to use cd command history to infer the current directory
  // This handles: cd /usr/local/sankuai -> cd .. -> prompt shows "local"
  if (hasRecentRelativeCd && !detectedPath && cdGuess && promptGuess) {
    logger.info('Using cd history to infer path after relative cd', {
      lastAbsoluteCd: cdGuess,
      currentPrompt: promptGuess,
    });

    // If the last absolute cd's parent directory name matches prompt,
    // then we're in that parent directory
    const parentPath = cdGuess.substring(0, cdGuess.lastIndexOf('/'));
    const parentDirName = parentPath.split('/').pop() || parentPath;

    if (parentDirName === promptGuess) {
      // Match! We're in the parent directory of the last cd
      detectedPath = parentPath || '/';
      detectionSource = 'inferred-parent-from-cd-and-prompt';
      hasRecentRelativeCd = false; // Mark as resolved
      logger.debug('✅ Inferred parent directory', {
        lastCd: cdGuess,
        parentPath,
        parentDirName,
        currentPrompt: promptGuess,
        inferred: detectedPath,
      });
    } else if (
      cdGuess === `/${promptGuess}` ||
      cdGuess.endsWith(`/${promptGuess}`)
    ) {
      // The prompt matches the last component of the cd command (cd /a/b -> cd .. -> prompt "a")
      // This shouldn't happen if we have cd .., but handle it anyway
      const parts = cdGuess.split('/').filter(p => p);
      if (parts.length > 1) {
        const reconstructed = '/' + parts.slice(0, -1).join('/');
        detectedPath = reconstructed;
        detectionSource = 'inferred-sibling-from-cd-and-prompt';
        hasRecentRelativeCd = false;
        logger.debug('✅ Inferred sibling directory', {
          inferred: detectedPath,
        });
      }
    }
  }

  if (!detectedPath && relativeCdGuess) {
    const resolved = resolveRelativeCd(
      relativeCdGuess,
      lastKnownAbsolutePath.value,
      remoteHomeDir.value
    );
    if (resolved) {
      detectedPath = resolved;
      detectionSource = 'relative-cd-resolved';
      hasRecentRelativeCd = false;
      logger.debug('✅ Resolved relative cd with last known path', {
        relativeCd: relativeCdGuess,
        basePath: lastKnownAbsolutePath.value,
        resolved,
      });
    }
  }

  // Final fallback
  if (!detectedPath) {
    detectedPath = lastKnownAbsolutePath.value || remoteHomeDir.value || '.';
    detectionSource = lastKnownAbsolutePath.value
      ? 'fallback-last-known'
      : 'fallback-home';
  }

  // Update the UI with the detected path
  currentRemotePath.value = detectedPath;
  lastPathDetectionSource.value = detectionSource;
  if (currentRemotePath.value.startsWith('/')) {
    lastKnownAbsolutePath.value = normalizeRemotePath(currentRemotePath.value);
  }
  logger.info('Path detection complete', {
    finalPath: currentRemotePath.value,
    source: detectionSource,
  });
};

/**
 * Handle file drop - Process uploads asynchronously
 * Non-blocking: returns immediately, all processing happens in background
 */
const handleFileDrop = (paths: string[]) => {
  // Show dashboard immediately and switch to uploads tab
  showDashboard.value = true;
  activeDashboardTab.value = 'uploads';

  // Ensure terminal retains focus
  nextTick(() => {
    terminal?.focus();
  });

  // Detect path and process files in background (fire-and-forget)
  // Do NOT await here to keep the function non-blocking
  (async () => {
    try {
      // Detect current path
      await detectRemotePath();

      logger.info('Files dropped', {
        paths,
        targetPath: currentRemotePath.value,
        home: remoteHomeDir.value,
      });

      // Process all files asynchronously without blocking
      paths.forEach(path => {
        processFileUpload(path);
      });
    } catch (err) {
      logger.error('Failed to process dropped files', err);
    }
  })();
};

/**
 * Process a single file upload asynchronously
 */
/**
 * Process a single file upload asynchronously without blocking
 * Fire-and-forget pattern: start upload in background and return immediately
 */
const processFileUpload = async (path: string) => {
  const fileName = path.split('/').pop() || path;
  const taskId = addUploadTask(fileName);

  // Prepare upload parameters in this async function
  // But do NOT await the actual upload - let it run in background

  try {
    let targetDir = currentRemotePath.value || '';
    let pathSource = 'unknown';

    // 1. Handle home expansion
    if (remoteHomeDir.value) {
      if (targetDir === '~') {
        targetDir = remoteHomeDir.value;
        pathSource = 'home-expansion';
      } else if (targetDir.startsWith('~/')) {
        targetDir = targetDir.replace('~', remoteHomeDir.value);
        pathSource = 'home-expansion';
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
      pathSource = 'resolved-relative-to-last-known';
    }

    // 2. Robust path normalization
    let remoteFilePath = '';

    if (!targetDir || targetDir === '.' || targetDir === '') {
      if (lastKnownAbsolutePath.value) {
        targetDir = lastKnownAbsolutePath.value;
        pathSource = 'last-known-absolute';
      } else {
        targetDir = remoteHomeDir.value || '.';
        pathSource = 'fallback-home';
      }
    } else {
      if (targetDir.startsWith('/')) {
        pathSource = 'absolute-path';
      } else {
        pathSource = 'relative-path';
      }
    }

    // 3. Build final SFTP path
    if (targetDir === '.') {
      remoteFilePath = fileName;
    } else {
      remoteFilePath = targetDir.endsWith('/')
        ? `${targetDir}${fileName}`
        : `${targetDir}/${fileName}`;
    }

    updateUploadTask(taskId, {
      status: 'uploading',
      progress: 10,
      message: `Preparing upload...`,
      remotePath: remoteFilePath,
    });

    logger.info('Path resolution', {
      originalPath: currentRemotePath.value,
      resolvedPath: remoteFilePath,
      source: pathSource,
      detectionMethod: lastPathDetectionSource.value,
      fileName,
    });

    // Start upload in background
    // Backend will handle the streaming and emit progress events
    invoke('upload_file_sftp', {
      sessionId: props.sessionId,
      taskId,
      localPath: path,
      remotePath: remoteFilePath,
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

/**
 * Establish connection via session store and API
 */
const connectSession = async (cols: number, rows: number): Promise<void> => {
  if (!props.sessionId) {
    throw new Error('sessionId is required');
  }

  if (sessionStore.hasSession(props.sessionId)) {
    return;
  }

  try {
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

      await sessionStore.createSSHSession(
        props.sessionId,
        props.sessionId,
        serverName,
        props.ip || '',
        props.port || 22,
        props.username || '',
        props.password || '',
        cols,
        rows
      );
    }

    // Retrieve buffered initial output (e.g., welcome banner, login prompts)
    // Wait for the backend to complete the initial buffering phase.
    // A delay of ~2 seconds ensures the full initial sequence is captured for SSH.
    if (props.tabType !== 'terminal') {
      await new Promise(resolve => setTimeout(resolve, 2100));

      const bufferedOutput = await sessionApi.getBufferedSSHOutput(
        props.sessionId
      );
      if (terminal && bufferedOutput.length > 0) {
        logger.info('Writing buffered SSH output to terminal', {
          chunks: bufferedOutput.length,
        });
        for (const chunk of bufferedOutput) {
          terminal.write(chunk.output);
          lastSeq = Math.max(lastSeq, chunk.seq);
        }

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
      } else {
        logger.debug('No buffered SSH output available');
      }
    }
  } catch (error) {
    logger.error('Connection failed', error);
    throw error;
  }
};

/**
 * Disconnect session
 */
const disconnectSession = async (): Promise<void> => {
  if (!props.sessionId) return;

  try {
    // Session store handles the specific disconnect logic based on session type
    await sessionStore.disconnectSession(props.sessionId);
  } catch (error) {
    logger.error('Disconnect failed', error);
  }
};

/**
 * Cleanup terminal resources
 */
const cleanupResources = async (): Promise<void> => {
  if (props.sessionId) {
    await disconnectSession();
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

  if (unlistenDrag) await unlistenDrag();
  if (unlistenDragEnter) await unlistenDragEnter();
  if (unlistenDragLeave) await unlistenDragLeave();
  if (unlistenUpload) await unlistenUpload();
};

/**
 * Expose cleanup method to parent
 */
defineExpose({
  cleanupResources,
});

/**
 * Component lifecycle
 */
onMounted(async () => {
  logger.info('Terminal component mounted', { sessionId: props.sessionId });
  await setupStatusListener();

  // Listen for background upload progress
  unlistenUpload = await listen<UploadProgressPayload>(
    'upload-progress',
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
    theme: TERMINAL_CONFIG.THEME,
  });

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
  terminalRef.value?.addEventListener('contextmenu', async (e: MouseEvent) => {
    e.preventDefault();
    if (terminal?.hasSelection()) {
      const selection = terminal.getSelection();
      if (selection) {
        await navigator.clipboard.writeText(selection);
        terminal.clearSelection();
      }
    } else {
      try {
        const text = await navigator.clipboard.readText();
        if (text && terminal) {
          terminal.paste(text);
        }
      } catch (err) {
        logger.error('Right-click paste failed', err);
      }
    }
  });

  // Handle keyboard shortcuts and allow global app shortcuts to bubble up
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    if (event.type !== 'keydown') return true;

    const isMac = navigator.userAgent.includes('Mac');
    const isControlKey = isMac ? event.metaKey : event.ctrlKey;
    const key = event.key.toLowerCase();

    // 1. Allow global app shortcuts to bubble up to the window
    // This includes Cmd+T, Cmd+Shift+T, Cmd+W, Cmd+K, Cmd+, and Cmd+Shift+P
    if (
      isControlKey &&
      (['t', 'w', 'k', ','].includes(key) || event.shiftKey)
    ) {
      return false;
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

    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyL') {
      terminal?.clear();
      return false;
    }

    return true;
  });

  // Use ResizeObserver for robust layout management
  const resizeObserver = new ResizeObserver(() => {
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

  // Initial connection is handled when a valid `sessionId` is provided

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
    });
  });

  // Handle window resize
  const handleResize = (): void => {
    if (fitAddon) {
      fitAddon.fit();
    }
  };
  window.addEventListener('resize', handleResize);

  /**
   * Setup SSH output event listener before connection
   */
  const setupSSHOutputListener = async (sessionId: string): Promise<void> => {
    if (unlistenFn) {
      await unlistenFn();
    }

    unlistenFn = await listen(`ssh-output-${sessionId}`, (event: any) => {
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
   * Watch sessionId changes and connect/disconnect accordingly
   */
  watch(
    () => props.sessionId,
    async (newSessionId, oldSessionId) => {
      if (newSessionId && newSessionId !== oldSessionId) {
        lastSeq = 0;

        try {
          // ✅ Register listener BEFORE connection to catch welcome message
          await setupSSHOutputListener(newSessionId);

          if (terminal) {
            // First, ensure we have the correct size before connecting
            if (fitAddon) {
              fitAddon.fit();
            }

            await connectSession(terminal.cols, terminal.rows);

            // Re-sync after a short delay to ensure backend is ready and listener is active
            setTimeout(() => {
              if (fitAddon && props.sessionId === newSessionId) {
                fitAddon.fit();
                emit(`ssh-resize-${newSessionId}`, {
                  cols: terminal!.cols,
                  rows: terminal!.rows,
                });
              }
            }, 500);
          }
        } catch (error) {
          logger.error('Connection failed', error);
        }
      }
    },
    { immediate: true }
  );

  /**
   * Handle terminal input
   */
  // Key sequence mapping for terminal input
  /**
   * Terminal input handling
   * Using onData instead of onKey to properly handle IME (Chinese input)
   */
  let inputBuffer = '';
  let inputFlushTimeout: ReturnType<typeof setTimeout> | null = null;

  const flushInput = async (): Promise<void> => {
    if (!inputBuffer || !props.sessionId) return;

    const data = inputBuffer;
    inputBuffer = '';

    try {
      await emit(`ssh-input-${props.sessionId}`, { input: data });
    } catch (error) {
      logger.error('Input emit failed', error);
    }
  };

  terminal.onData(async (data: string) => {
    const hasSession =
      props.sessionId && sessionStore.hasSession(props.sessionId);

    if (hasSession) {
      inputBuffer += data;

      if (inputBuffer.length > INPUT_BUFFER_CONFIG.FLUSH_THRESHOLD) {
        clearTimeout(inputFlushTimeout!);
        await flushInput();
      } else {
        clearTimeout(inputFlushTimeout!);
        inputFlushTimeout = setTimeout(
          flushInput,
          INPUT_BUFFER_CONFIG.FLUSH_DELAY_MS
        );
      }
    } else if (data.length === 1 && data >= ' ') {
      // No active session: echo printable characters locally
      terminal?.write(data);
    }
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

  /**
   * Cleanup on unmount
   * With KeepAlive, this only fires when the tab is truly closed or the cache is purged.
   */
  onUnmounted(async () => {
    window.removeEventListener('resize', handleResize);
    resizeObserver.disconnect();
    if (statusUnlisten) {
      statusUnlisten();
      statusUnlisten = null;
    }
    await cleanupResources();
  });
});
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
    />
    <div ref="terminalRef" class="terminal-container" />

    <!-- Drag and Drop Overlay -->
    <div v-if="isDragging" class="drag-drop-overlay">
      <div class="overlay-content">
        <span class="icon">📂</span>
        <p>Drop files here to upload</p>
        <div class="target-path-display">
          <div class="path-label">Target Directory:</div>
          <input
            v-model="currentRemotePath"
            class="bg-black/40 border border-white/20 rounded px-3 py-1 text-yellow-400 font-mono text-sm focus:border-yellow-400 outline-none w-72 text-center pointer-events-auto"
            placeholder="(auto-detected path)"
            @click.stop
            @mousedown.stop
          />
          <div v-if="!currentRemotePath" class="path-status warning">
            ⚠️ No path detected - will use home directory
          </div>
          <div
            v-else-if="currentRemotePath.startsWith('/')"
            class="path-status success"
          >
            ✓ Absolute path
          </div>
          <div v-else class="path-status info">
            ℹ️ Relative path - will upload to home + path
          </div>
        </div>
        <div class="overlay-tip">
          💡 Edit path above if needed before dropping files
        </div>
      </div>
    </div>

    <div v-if="showSearch" class="terminal-search-box">
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        type="text"
        placeholder="Search..."
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
  background-color: #1e1e1e;
  padding: 10px;
  box-sizing: border-box;
  position: relative;
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

.path-status {
  font-size: 12px;
  margin-top: 6px;
  padding: 6px 8px;
  border-radius: 3px;
  font-family: monospace;
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
</style>
