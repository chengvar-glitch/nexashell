/**
 * Transfer-queue composable for the standalone file-manager window.
 *
 * Tracks the session's uploads/downloads in a local queue and exposes
 * start/resume/pause/cancel actions. Progress is driven by the backend's
 * per-session broadcasts (`ssh-upload-progress-{sid}` / `ssh-download-progress-{sid}`),
 * exactly like the main window's sidebar queue. The queue is seeded from the
 * initiating action in this window; entries are matched by `taskId`.
 */

import { ref, readonly, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { createLogger } from '@/core/utils/logger';
import type { UploadTask, SftpEntry } from '@/core/types';

const logger = createLogger('TRANSFER_QUEUE');

interface UploadProgressPayload {
  taskId: string;
  sessionId: string;
  progress: number;
  uploadedBytes: number;
  totalBytes: number;
  status: 'uploading' | 'downloading' | 'paused' | 'success' | 'error' | 'cancelled';
  message: string;
  speed: number;
  error?: string;
}

/**
 * Create a new transfer queue bound to a session. Call `dispose()` on unmount
 * to release the listeners.
 */
export function useTransferQueue(sessionId: Ref<string>) {
  const tasks = ref<UploadTask[]>([]);

  let unlistenUpload: UnlistenFn | null = null;
  let unlistenDownload: UnlistenFn | null = null;

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
    const existing = tasks.value.find(t => t.id === payload.taskId);
    const status = payload.status as UploadTask['status'];
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
      error: payload.error || undefined,
    });
  };

  const setupListeners = async () => {
    const sid = sessionId.value;
    if (!sid) return;
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
    const taskId = `transfer-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    addTask({
      id: taskId,
      fileName,
      direction: 'upload',
      status: 'uploading',
      progress: 0,
      message: 'Preparing upload...',
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
      updateTask(taskId, { status: 'error', progress: 0, message: `Failed: ${msg}`, error: msg });
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
    const taskId = `transfer-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    addTask({
      id: taskId,
      fileName: entry.name,
      direction: 'download',
      status: 'downloading',
      progress: 5,
      message: 'Preparing download...',
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
      updateTask(taskId, { status: 'error', message: `Failed: ${msg}`, error: msg });
      logger.error('Failed to start download', err);
    }
  };

  const pause = async (taskId: string) => {
    updateTask(taskId, { status: 'paused', message: 'Pausing...' });
    try {
      await invoke('pause_upload', { sessionId: sessionId.value, taskId });
      updateTask(taskId, { message: 'Paused' });
    } catch (err) {
      logger.error('Failed to pause upload', err);
      updateTask(taskId, { message: 'Failed to pause' });
    }
  };

  const resume = async (taskId: string) => {
    updateTask(taskId, { status: 'uploading', message: 'Resuming...' });
    try {
      await invoke('resume_upload', { sessionId: sessionId.value, taskId });
    } catch (err) {
      logger.error('Failed to resume upload', err);
      updateTask(taskId, { message: 'Failed to resume' });
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
    }
    // Optimistic remove.
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
  };

  return {
    tasks: readonly(tasks),
    setupListeners,
    startUpload,
    startDownload,
    pause,
    resume,
    cancel,
    clearCompleted,
    dispose,
  };
}
