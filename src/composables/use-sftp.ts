/**
 * SFTP browser composable.
 *
 * Wraps the Tauri `sftp_*` invoke commands and the per-session
 * `ssh-download-progress-{sid}` event stream. A SFTP browser uses this to list
 * directories and kick off downloads; the surrounding transfer progress is
 * surfaced through the shared upload/download task queue (see
 * `use-sftp-task-queue` in the feature layer).
 *
 * Path handling is deliberately tolerant of Windows hosts: OpenSSH for Windows
 * virtualizes drive roots as `/C:/`, `/D:/`, and native backslash paths
 * (`C:\Users`) are normalized to the forward-slash SFTP form so browsing,
 * mkdir, rename and uploads keep working against either platform.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ref, readonly, type Ref } from 'vue';
import { createLogger } from '@/core/utils/logger';
import type { SftpEntry } from '@/core/types';

const logger = createLogger('SFTP');

export interface SftpState {
  currentPath: Ref<string>;
  entries: Ref<SftpEntry[]>;
  loading: Ref<boolean>;
  error: Ref<string>;
  stack: Ref<string[]>;
}

/** Remote platform category (`windows` | `macos` | `linux` | `unknown`). */
export type RemotePlatform = 'windows' | 'macos' | 'linux' | 'unknown';

/** True when the path looks like a native Windows absolute path (`C:\` / `C:/`). */
function isWindowsNativeAbsolute(path: string): boolean {
  return /^[A-Za-z]:[\\/]/.test(path);
}

/** True when the path looks like an OpenSSH virtual drive root (`/C:/`). */
function isVirtualDrivePath(path: string): boolean {
  return /^\/[A-Za-z]:(\/|$)/.test(path);
}

/** Convert backslashes to forward slashes (SFTP always uses `/`). */
function toForwardSlashes(path: string): string {
  return path.replace(/\\/g, '/');
}

/**
 * Normalize any path the UI hands us into the canonical SFTP form:
 * - `/C:/Users` virtual drive roots are kept as-is;
 * - `C:\Users` and `C:/Users` are re-written to `/C:/Users`;
 * - POSIX absolute paths keep their leading slash;
 * - relative paths gain a leading slash;
 * - empty/`.` becomes `/`.
 */
function normalizePath(path: string): string {
  if (!path || path === '.') return '/';
  const trimmed = path.trim();
  if (!trimmed) return '/';

  // Native Windows absolute path -> virtual `/C:/...` form.
  if (isWindowsNativeAbsolute(trimmed)) {
    return toForwardSlashes(trimmed).replace(/^([A-Za-z]):/, '/$1:');
  }
  // Already-virtual drive path stays as-is.
  if (isVirtualDrivePath(trimmed)) {
    return toForwardSlashes(trimmed);
  }

  const forward = toForwardSlashes(trimmed);
  const looksAbsolute = forward.startsWith('/');
  if (looksAbsolute) {
    // Collapse duplicate slashes but preserve the root "/".
    const collapsed = forward.replace(/\/+/g, '/').replace(/\/$/, '');
    return collapsed === '' ? '/' : collapsed;
  }
  return `/${forward}`;
}

/**
 * Return the parent path of an absolute path, or null when already at root.
 * Handles both POSIX (`/a/b` -> `/a`) and Windows drive paths (`/C:/Users` ->
 * `/C:/`, and `C:\Users` -> `C:\`).
 */
export function parentOfPath(path: string): string | null {
  const normalized = normalizePath(path);
  if (normalized === '/') return null;

  // `/C:/Users` -> `/C:/`; the drive root `/C:/` itself sits under `/`.
  const driveRootMatch = normalized.match(/^\/[A-Za-z]:\/?$/);
  if (driveRootMatch) return '/';

  const idx = normalized.lastIndexOf('/');
  if (idx <= 0) return '/';
  const parent = normalized.substring(0, idx);
  // `/C:` -> `/C:/` keeps drive roots navigable as directories.
  return /^\/[A-Za-z]:$/.test(parent) ? `${parent}/` : parent;
}

/**
 * Join a name onto a base directory using the platform-appropriate separator,
 * producing the canonical forward-slash SFTP path.
 */
function joinPath(base: string, name: string): string {
  const normalizedBase = normalizePath(base);
  if (normalizedBase === '/') {
    return `/${name}`;
  }
  return `${normalizedBase.replace(/\/$/, '')}/${name}`;
}

/**
 * Create an instance of the SFTP browser state bound to a session id. Each
 * component (or session) gets its own instance to avoid cross-session state
 * leakage. Call `dispose()` on unmount to release the event listener.
 */
export function useSftp(sessionId: Ref<string>) {
  const currentPath = ref<string>('/');
  const entries = ref<SftpEntry[]>([]);
  const loadedPath = ref<string | null>(null);
  const loading = ref<boolean>(false);
  const error = ref<string>('');
  const stack = ref<string[]>([]);
  const platform = ref<RemotePlatform>('unknown');
  let unlisten: UnlistenFn | null = null;
  // Monotonic request id so a slow in-flight listing cannot clobber state
  // (or clear `loading`) after a newer navigation has started.
  let requestSeq = 0;
  let platformProbeStarted = false;

  const go = async (path: string): Promise<boolean> => {
    const target = normalizePath(path);
    const sid = sessionId.value;
    if (!sid) {
      error.value = 'Not connected';
      return false;
    }
    const seq = ++requestSeq;
    loading.value = true;
    error.value = '';
    // Guard against a hung `sftp_list_dir` invoke leaving `loading` stuck, which
    // would dim the list and disable pointer events forever.
    const watchdog = setTimeout(() => {
      if (seq === requestSeq) {
        loading.value = false;
      }
    }, 15000);
    try {
      const result = await invoke<SftpEntry[]>('sftp_list_dir', {
        sessionId: sid,
        path: target,
      });
      clearTimeout(watchdog);
      // A newer request has superseded this one — do not touch state.
      if (seq !== requestSeq) return false;
      currentPath.value = target;
      loadedPath.value = target;
      entries.value = result;
      loading.value = false;
      return true;
    } catch (e) {
      clearTimeout(watchdog);
      logger.error('sftp_list_dir failed', e);
      if (seq !== requestSeq) return false;
      loading.value = false;
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    }
  };

  const navigate = async (path: string): Promise<boolean> => {
    const ok = await go(path);
    if (ok) {
      stack.value.push(currentPath.value);
    }
    return ok;
  };

  const goUp = async (): Promise<boolean> => {
    const parent = parentOfPath(currentPath.value);
    if (parent === null) return false;
    return go(parent);
  };

  const goHome = async (home = '/'): Promise<boolean> => {
    return go(home);
  };

  const refresh = async (): Promise<boolean> => {
    return go(currentPath.value);
  };

  const mkdir = async (name: string): Promise<boolean> => {
    const sid = sessionId.value;
    if (!sid || !name) return false;
    try {
      const target = joinPath(currentPath.value, name);
      await invoke('sftp_mkdir', {
        sessionId: sid,
        remotePath: target,
      });
      await refresh();
      return true;
    } catch (e) {
      logger.error('sftp_mkdir failed', e);
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    }
  };

  const remove = async (entry: SftpEntry): Promise<boolean> => {
    const sid = sessionId.value;
    if (!sid) return false;
    try {
      await invoke('sftp_remove', {
        sessionId: sid,
        remotePath: normalizePath(entry.path),
        isDir: entry.isDir,
      });
      await refresh();
      return true;
    } catch (e) {
      logger.error('sftp_remove failed', e);
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    }
  };

  const rename = async (entry: SftpEntry, newName: string): Promise<boolean> => {
    const sid = sessionId.value;
    if (!sid || !newName || newName === entry.name) return false;
    try {
      const newPath = joinPath(currentPath.value, newName);
      await invoke('sftp_rename', {
        sessionId: sid,
        oldPath: normalizePath(entry.path),
        newPath,
      });
      await refresh();
      return true;
    } catch (e) {
      logger.error('sftp_rename failed', e);
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    }
  };

  /**
   * Probe (once) the remote platform so the browser can tailor path handling.
   * Non-fatal: on failure the platform stays `unknown` and POSIX defaults apply.
   */
  const probePlatform = async (): Promise<RemotePlatform> => {
    if (platformProbeStarted) return platform.value;
    platformProbeStarted = true;
    const sid = sessionId.value;
    if (!sid) return 'unknown';
    try {
      const result = await invoke<string>('sftp_probe_platform', {
        sessionId: sid,
      });
      const p = (['windows', 'macos', 'linux', 'unknown'] as const).includes(
        result as RemotePlatform
      )
        ? (result as RemotePlatform)
        : 'unknown';
      platform.value = p;
      return p;
    } catch (e) {
      logger.warn('sftp_probe_platform failed', e);
      return 'unknown';
    }
  };

  /**
   * Register a handler for this session's download progress events. Returns an
   * unlisten function; only one handler is registered per instance.
   */
  const onDownloadProgress = async (
    handler: (payload: {
      taskId: string;
      progress: number;
      uploadedBytes: number;
      totalBytes: number;
      status: string;
      message: string;
      speed: number;
      error?: string;
    }) => void
  ): Promise<void> => {
    const sid = sessionId.value;
    if (!sid) return;
    if (unlisten) {
      await unlisten();
      unlisten = null;
    }
    unlisten = await listen(
      `ssh-download-progress-${sid}`,
      (event: { payload?: unknown }) => {
        try {
          const payload = event.payload as {
            taskId: string;
            progress: number;
            uploadedBytes: number;
            totalBytes: number;
            status: string;
            message: string;
            speed: number;
            error?: string;
          };
          handler(payload);
        } catch (e) {
          logger.error('download progress handler failed', e);
        }
      }
    );
  };

  const dispose = async (): Promise<void> => {
    if (unlisten) {
      await unlisten();
      unlisten = null;
    }
  };

  return {
    currentPath,
    entries,
    loadedPath,
    loading,
    error,
    stack,
    platform,
    go,
    list: go,
    refresh,
    navigate,
    goUp,
    goHome,
    mkdir,
    remove,
    rename,
    onDownloadProgress,
    probePlatform,
    dispose,
  };
}

/** Alias for consumers that want a stable, immutable view of browsing state. */
export function useSftpBrowseState(sftp: ReturnType<typeof useSftp>) {
  return {
    currentPath: readonly(sftp.currentPath),
    entries: readonly(sftp.entries),
    loading: readonly(sftp.loading),
    error: readonly(sftp.error),
  };
}

export { normalizePath };