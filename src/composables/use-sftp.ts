/**
 * SFTP browser composable.
 *
 * Wraps the Tauri `sftp_*` invoke commands and the per-session
 * `ssh-download-progress-{sid}` event stream. A SFTP browser uses this to list
 * directories and kick off downloads; the surrounding transfer progress is
 * surfaced through the shared upload/download task queue (see
 * `use-sftp-task-queue` in the feature layer).
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

function normalizePath(path: string): string {
  if (!path || path === '') return '/';
  if (path === '.') return '/';
  const trimmed = path.trim();
  if (!trimmed.startsWith('/')) return `/${trimmed}`;
  return trimmed;
}

/**
 * Return the parent path of an absolute path, or null when already at root.
 */
export function parentOfPath(path: string): string | null {
  const normalized = normalizePath(path);
  if (normalized === '/') return null;
  const idx = normalized.lastIndexOf('/');
  if (idx <= 0) return '/';
  return normalized.substring(0, idx);
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
  let unlisten: UnlistenFn | null = null;
  // Monotonic request id so a slow in-flight listing cannot clobber state
  // (or clear `loading`) after a newer navigation has started.
  let requestSeq = 0;

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
      const target = `${currentPath.value === '/' ? '' : currentPath.value}/${name}`;
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
        remotePath: entry.path,
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
      const newPath = `${
        currentPath.value === '/' ? '' : currentPath.value
      }/${newName}`;
      await invoke('sftp_rename', {
        sessionId: sid,
        oldPath: entry.path,
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
