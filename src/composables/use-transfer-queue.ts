/**
 * Transfer-queue composable for the standalone file-manager window.
 *
 * Tracks the session's uploads/downloads in a local queue and exposes
 * start/resume/pause/cancel actions. Progress is driven by the backend's
 * per-session broadcasts (`ssh-upload-progress-{sid}` / `ssh-download-progress-{sid}`),
 * exactly like the main window's sidebar queue. The queue is seeded from the
 * initiating action in this window; entries are matched by `taskId`.
 */

import { ref, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { createLogger } from '@/core/utils/logger';
import { i18n } from '@/core/i18n';
import type { UploadTask, SftpEntry, UploadProgressPayload } from '@/core/types';

const logger = createLogger('TRANSFER_QUEUE');

/**
 * Create a new transfer queue bound to a session. Call `dispose()` on unmount
 * to release the listeners.
 */
export function useTransferQueue(sessionId: Ref<string>) {
  // Global i18n instance (not useI18n) so the composable keeps working
  // outside a component setup context (e.g. unit tests).
  const t = (key: string, params?: Record<string, unknown>): string =>
    params ? i18n.global.t(key, params) : i18n.global.t(key);
  const tasks = ref<UploadTask[]>([]);

  let unlistenUpload: UnlistenFn | null = null;
  let unlistenDownload: UnlistenFn | null = null;

  /**
   * Task ids that WE cancelled from this window. cancel() removes the row
   * optimistically, but the backend still broadcasts a final 'cancelled'
   * progress event which applyProgress would otherwise rebuild via its
   * unknown-taskId branch. We skip the rebuild for these and drop the id once
   * the terminal event arrives.
   */
  const cancelledByUs = new Set<string>();

  const updateTask = (taskId: string, updates: Partial<UploadTask>) => {
    const found = tasks.value.find(t => t.id === taskId);
    if (!found) return;
    Object.assign(found, updates);
  };

  const addTask = (
    task: Omit<UploadTask, 'timestamp'> & { timestamp?: number }
  ): string => {
    const id = task.id;
    if (!tasks.value.some(t => t.id === id)) {
      tasks.value.unshift({ timestamp: Date.now(), ...task });
    }
    return id;
  };

  const applyProgress = (payload: UploadProgressPayload, direction: UploadTask['direction']) => {
    // Downloads: the backend broadcasts its own statuses, but non-terminal
    // ones must render as 'downloading' on the queue row regardless of what
    // the backend labels them.
    const status = (payload.status === 'success' || payload.status === 'error' || payload.status === 'cancelled'
      ? payload.status
      : direction === 'download'
        ? 'downloading'
        : payload.status) as UploadTask['status'];
    const eta =
      payload.speed > 0
        ? (payload.totalBytes - payload.uploadedBytes) / payload.speed
        : undefined;
    // A final event for a task this window cancelled: we already removed it
    // optimistically, so do NOT rebuild it. Drop the marker so later unrelated
    // events are unaffected.
    if (cancelledByUs.has(payload.taskId)) {
      if (status === 'success' || status === 'error' || status === 'cancelled') {
        cancelledByUs.delete(payload.taskId);
      }
      return;
    }
    const existing = tasks.value.find(t => t.id === payload.taskId);
    if (!existing) {
      // A progress event for an unknown task (e.g. started in the main window):
      // keep a lightweight entry so state stays consistent across windows.
      addTask({
        id: payload.taskId,
        fileName: '',
        direction,
        status:
          status === 'success' || status === 'error' || status === 'cancelled'
            ? status
            : direction === 'download'
              ? 'downloading'
              : 'uploading',
        progress: Math.floor(payload.progress),
        message: payload.message,
        fileSize: payload.totalBytes,
        uploadedBytes: payload.uploadedBytes,
        speed: payload.speed,
        eta,
        error: payload.error || undefined,
      });
      return;
    }
    updateTask(payload.taskId, {
      direction,
      status,
      progress: Math.floor(payload.progress),
      message: payload.message,
      uploadedBytes: payload.uploadedBytes,
      fileSize: payload.totalBytes || existing.fileSize,
      speed: payload.speed,
      eta,
      error: payload.error || undefined,
    });
  };

  const setupListeners = async () => {
    const sid = sessionId.value;
    if (!sid) return;
    // Idempotent: tear down any previous registration first so a second call
    // cannot orphan the old listeners.
    if (unlistenUpload) {
      unlistenUpload();
      unlistenUpload = null;
    }
    if (unlistenDownload) {
      unlistenDownload();
      unlistenDownload = null;
    }
    unlistenUpload = await listen<UploadProgressPayload>(
      `ssh-upload-progress-${sid}`,
      event => applyProgress(event.payload, 'upload')
    );
    unlistenDownload = await listen<UploadProgressPayload>(
      `ssh-download-progress-${sid}`,
      event => applyProgress(event.payload, 'download')
    );
  };

  const startUpload = async (localPath: string, remotePath: string, fileName: string): Promise<string | null> => {
    const sid = sessionId.value;
    if (!sid) return null;
    const taskId = `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
    addTask({
      id: taskId,
      fileName,
      direction: 'upload',
      status: 'uploading',
      progress: 0,
      message: t('upload.preparingUpload'),
      remotePath,
    });
    try {
      await invoke('upload_file_sftp', {
        sessionId: sid,
        taskId,
        localPath,
        remotePath,
      });
      return taskId;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      updateTask(taskId, { status: 'error', progress: 0, message: t('upload.failed', { msg }), error: msg });
      logger.error('Failed to start upload', err);
      return null;
    }
  };

  const startDownload = async (
    entry: SftpEntry,
    localPath: string
  ): Promise<void> => {
    const sid = sessionId.value;
    if (!sid) return;
    const taskId = `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
    addTask({
      id: taskId,
      fileName: entry.name,
      direction: 'download',
      status: 'downloading',
      progress: 5,
      message: t('upload.preparingDownload'),
      remotePath: entry.path,
      localPath,
      fileSize: entry.size,
    });
    try {
      await invoke('sftp_download_file', {
        sessionId: sid,
        taskId,
        remotePath: entry.path,
        localPath,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      updateTask(taskId, { status: 'error', message: t('upload.failed', { msg }), error: msg });
      logger.error('Failed to start download', err);
    }
  };

  const pause = async (taskId: string) => {
    const task = tasks.value.find(t => t.id === taskId);
    if (!task) return;
    // Pausing only makes sense for uploads; the backend has no
    // pause_download, so report the constraint instead of a silent no-op.
    if (task.direction !== 'upload') {
      updateTask(taskId, { message: t('upload.pauseUploadsOnly') });
      logger.warn('pause() called for a non-upload task', { taskId, direction: task.direction });
      return;
    }
    updateTask(taskId, { status: 'paused', message: t('upload.pausing') });
    try {
      await invoke('pause_upload', { sessionId: sessionId.value, taskId });
      updateTask(taskId, { message: t('upload.paused') });
    } catch (err) {
      logger.error('Failed to pause upload', err);
      updateTask(taskId, { message: t('upload.failedToPause') });
    }
  };

  const resume = async (taskId: string) => {
    const task = tasks.value.find(t => t.id === taskId);
    if (!task) return;
    // Resume must reflect the task's actual direction instead of hard-coding
    // 'uploading'; downloads cannot be resumed via resume_upload.
    if (task.direction !== 'upload') {
      updateTask(taskId, { message: t('upload.resumeUploadsOnly') });
      logger.warn('resume() called for a non-upload task', { taskId, direction: task.direction });
      return;
    }
    updateTask(taskId, { status: 'uploading', message: t('upload.resuming') });
    try {
      await invoke('resume_upload', { sessionId: sessionId.value, taskId });
    } catch (err) {
      logger.error('Failed to resume upload', err);
      updateTask(taskId, { message: t('upload.failedToResume') });
    }
  };

  const cancel = async (taskId: string) => {
    const task = tasks.value.find(t => t.id === taskId);
    const isDownload = task?.direction === 'download';
    try {
      await invoke(isDownload ? 'cancel_download' : 'cancel_upload', {
        sessionId: sessionId.value,
        taskId,
      });
    } catch (err) {
      logger.error('Failed to cancel transfer', err);
      // Re-insert a terminal error entry — the partial remote file may be
      // left behind when the backend cancellation truly failed.
      tasks.value = [
        ...tasks.value,
        task
          ? { ...task, status: 'error' as const, message: t('upload.failedToCancel') }
          : {
              id: taskId,
              fileName: t('upload.failedToCancel'),
              status: 'error' as const,
              progress: 0,
              message: t('upload.failedToCancel'),
              timestamp: Date.now(),
            },
      ];
      return;
    }
    // Remember this cancellation so the final 'cancelled' broadcast does not
    // resurrect the row, then remove it optimistically.
    cancelledByUs.add(taskId);
    tasks.value = tasks.value.filter(t => t.id !== taskId);
  };

  const clearCompleted = () => {
    tasks.value = tasks.value.filter(
      t => t.status === 'uploading' || t.status === 'downloading' || t.status === 'paused'
    );
  };

  const dispose = () => {
    if (unlistenUpload) {
      void unlistenUpload();
      unlistenUpload = null;
    }
    if (unlistenDownload) {
      void unlistenDownload();
      unlistenDownload = null;
    }
    cancelledByUs.clear();
  };

  return {
    tasks,
    setupListeners,
    startUpload,
    startDownload,
    pause,
    resume,
    cancel,
    clearCompleted,
    dispose,
    // Lower-level access for callers that resolve the target path themselves
    // (e.g. the main-window upload flow with its preview dialog).
    addTask,
    updateTask,
  };
}
