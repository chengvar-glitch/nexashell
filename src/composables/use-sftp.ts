/**
 * SFTP browser composable.
 *
 * Wraps the Tauri `sftp_*` invoke commands and the per-session
 * `ssh-download-progress-{sid}` event stream. A SFTP browser uses this to list
 * directories and kick off downloads; the surrounding transfer progress is
 * surfaced through the shared upload/download task queue in the feature layer.
 *
 * Path handling is deliberately tolerant of Windows hosts: OpenSSH for Windows
 * virtualizes drive roots as `/C:/`, `/D:/`, and native backslash paths
 * (`C:\Users`) are normalized to the forward-slash SFTP form so browsing,
 * mkdir, rename and uploads keep working against either platform.
 */

import { invoke } from '@tauri-apps/api/core';
import { ref, type Ref } from 'vue';
import { createLogger } from '@/core/utils/logger';
import { i18n } from '@/core/i18n';
import type { SftpEntry } from '@/core/types';

const logger = createLogger('SFTP');

export interface SftpState {
  currentPath: Ref<string>;
  entries: Ref<SftpEntry[]>;
  loading: Ref<boolean>;
  error: Ref<string>;
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
 * leakage. Call `dispose()` on unmount to stop any in-flight list watchdog and
 * mark the instance disposed so it stops writing reactive state.
 */
export function useSftp(sessionId: Ref<string>) {
  const currentPath = ref<string>('/');
  const entries = ref<SftpEntry[]>([]);
  const loading = ref<boolean>(false);
  const error = ref<string>('');
  const platform = ref<RemotePlatform>('unknown');
  // Monotonic request id so a slow in-flight listing cannot clobber state
  // (or clear `loading`) after a newer navigation has started.
  let requestSeq = 0;
  let platformProbeStarted = false;
  // Single in-flight list watchdog, tracked at composable scope so dispose()
  // can clear it — a dismissed instance must not keep writing reactive state.
  let watchdog: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;

  const go = async (path: string): Promise<boolean> => {
    if (disposed) return false;
    const target = normalizePath(path);
    const sid = sessionId.value;
    if (!sid) {
      // Global i18n instance (not useI18n) so the composable keeps working
      // outside a component setup context (e.g. unit tests).
      error.value = i18n.global.t('sftp.notConnected');
      return false;
    }
    const seq = ++requestSeq;
    loading.value = true;
    error.value = '';
    // Guard against a hung `sftp_list_dir` invoke leaving `loading` stuck, which
    // would dim the list and disable pointer events forever.
    if (watchdog) clearTimeout(watchdog);
    watchdog = setTimeout(() => {
      if (seq === requestSeq && !disposed) {
        loading.value = false;
      }
    }, 15000);
    try {
      const result = await invoke<SftpEntry[]>('sftp_list_dir', {
        sessionId: sid,
        path: target,
      });
      if (watchdog) {
        clearTimeout(watchdog);
        watchdog = null;
      }
      if (disposed) return false;
      // A newer request has superseded this one — do not touch state.
      if (seq !== requestSeq) return false;
      currentPath.value = target;
      entries.value = result;
      loading.value = false;
      return true;
    } catch (e) {
      if (watchdog) {
        clearTimeout(watchdog);
        watchdog = null;
      }
      logger.error('sftp_list_dir failed', e);
      if (disposed || seq !== requestSeq) return false;
      loading.value = false;
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    }
  };

  const navigate = async (path: string): Promise<boolean> => {
    // History is intentionally not tracked here — the previous stack array
    // grew without bound and was never consumed, so navigation is now a plain
    // go() (browsers/consumers that need history manage it themselves).
    return go(path);
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
  const dispose = async (): Promise<void> => {
    disposed = true;
    if (watchdog) {
      clearTimeout(watchdog);
      watchdog = null;
    }
  };

  return {
    currentPath,
    entries,
    loading,
    error,
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
    probePlatform,
    dispose,
  };
}

export { normalizePath };