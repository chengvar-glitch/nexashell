<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import SftpBrowser from '@/components/connections/SftpBrowser.vue';
import { useTransferQueue } from '@/composables/use-transfer-queue';
import { createLogger } from '@/core/utils/logger';
import { formatBytes, formatSpeed, type SftpEntry, type UploadTask } from '@/core/types';
import {
  Upload,
  Trash2,
  Play,
  PauseCircle,
  XCircle,
  ArrowUp,
  ArrowDown,
  CheckCircle2,
  AlertCircle,
  Activity,
} from 'lucide-vue-next';

const logger = createLogger('FILE_MANAGER_WINDOW');
const { t } = useI18n();

// The session id is encoded in the window label ("file-manager-{sessionId}").
const appWindow = getCurrentWindow();
const label = appWindow.label;
const sessionId = ref<string>(
  label.startsWith('file-manager-') ? label.slice('file-manager-'.length) : ''
);

const disconnected = ref(false);
let disconnectUnlisten: UnlistenFn | null = null;

const transferQueue = useTransferQueue(sessionId);
const showTransfers = ref(true);

// Current remote directory of the embedded browser (kept in sync via emit).
const currentRemotePath = ref('/');
const refreshKey = ref(0);

onMounted(async () => {
  if (!sessionId.value) return;
  disconnectUnlisten = await listen(`ssh-disconnected-${sessionId.value}`, () => {
    disconnected.value = true;
  });
  await transferQueue.setupListeners();
});

onUnmounted(() => {
  if (disconnectUnlisten) {
    void disconnectUnlisten();
    disconnectUnlisten = null;
  }
  transferQueue.dispose();
});

const onCurrentPath = (path: string) => {
  currentRemotePath.value = path;
};

/** Pick one or more local files and upload them into the current remote dir. */
const onUpload = async () => {
  if (!sessionId.value) return;
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({ multiple: true });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    const base = currentRemotePath.value === '/' ? '' : currentRemotePath.value;
    for (const localPath of paths) {
      const fileName = localPath.split('/').pop() || localPath;
      await transferQueue.startUpload(localPath, `${base}/${fileName}`, fileName);
    }
    refreshKey.value += 1;
  } catch (err) {
    logger.error('Upload dialog failed', err);
  }
};

const downloadEntry = (entry: SftpEntry) => {
  if (!sessionId.value) return;
  (async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const localPath = await save({ defaultPath: entry.name });
    if (!localPath) return;
    await transferQueue.startDownload(entry, localPath);
  })();
};

const taskStatusIcon = (status: UploadTask['status']) => {
  switch (status) {
    case 'success':
      return CheckCircle2;
    case 'error':
      return AlertCircle;
    case 'cancelled':
      return XCircle;
    case 'paused':
      return PauseCircle;
    case 'downloading':
      return ArrowDown;
    case 'uploading':
      return ArrowUp;
    default:
      return Activity;
  }
};

const taskStatusColor = (status: UploadTask['status']) => {
  switch (status) {
    case 'success':
      return '#10b981';
    case 'error':
      return '#ef4444';
    case 'cancelled':
      return '#6b7280';
    case 'paused':
      return '#facc15';
    case 'downloading':
      return '#8b5cf6';
    case 'uploading':
      return '#3b82f6';
    default:
      return '#6b7280';
  }
};
</script>

<template>
  <div class="file-manager-root">
    <template v-if="sessionId">
      <div v-if="disconnected" class="fm-disconnected">
        <span class="dot" />
        {{ t('dashboard.disconnected') }}
      </div>

      <div v-else class="fm-layout">
        <div class="fm-main">
          <div class="fm-toolbar">
            <div class="fm-path">
              <span class="fm-path-text" :title="currentRemotePath">{{
                currentRemotePath
              }}</span>
            </div>
            <div class="fm-toolbar-actions">
              <button
                type="button"
                class="fm-btn primary"
                :title="t('dashboard.upload')"
                @click="onUpload"
              >
                <Upload :size="15" />
                <span>{{ t('dashboard.upload') }}</span>
              </button>
              <button
                type="button"
                class="fm-btn"
                :title="t('dashboard.transfersToggle')"
                @click="showTransfers = !showTransfers"
              >
                <Activity :size="15" />
              </button>
            </div>
          </div>

          <div class="fm-browser">
            <SftpBrowser
              :session-id="sessionId"
              :refresh-key="refreshKey"
              @current-path="onCurrentPath"
              @download="downloadEntry"
            />
          </div>
        </div>

        <!-- Transfer queue panel (right side) -->
        <aside v-if="showTransfers" class="fm-transfers">
          <div class="fm-transfers-header">
            <span class="fm-transfers-title">{{ t('dashboard.uploads') }}</span>
            <div class="fm-transfers-actions">
              <button
                type="button"
                class="icon-btn-sm"
                :title="t('dashboard.clearAll')"
                @click="transferQueue.clearCompleted()"
              >
                <Trash2 :size="13" />
              </button>
              <button
                type="button"
                class="icon-btn-sm"
                :title="t('dashboard.closePanel')"
                @click="showTransfers = false"
              >
                <XCircle :size="13" />
              </button>
            </div>
          </div>

          <div
            v-if="transferQueue.tasks.value.length === 0"
            class="fm-transfers-empty"
          >
            <Activity :size="20" />
            <p>{{ t('dashboard.noUploadTasks') }}</p>
          </div>
          <div v-else class="fm-transfers-list">
            <div
              v-for="task in transferQueue.tasks.value"
              :key="task.id"
              class="fm-task"
              :class="task.status"
            >
              <div class="fm-task-header">
                <component
                  :is="taskStatusIcon(task.status)"
                  :size="13"
                  class="fm-task-icon"
                  :style="{ color: taskStatusColor(task.status) }"
                />
                <span class="fm-task-name" :title="task.fileName || task.id">{{
                  task.fileName || 'Transfer'
                }}</span>
                <span class="fm-task-time">{{ Math.floor(task.progress) }}%</span>
              </div>
              <div class="fm-task-progress">
                <div
                  class="fm-task-progress-fill"
                  :class="`fill-${task.status}`"
                  :style="{ width: `${Math.min(100, task.progress)}%` }"
                />
              </div>
              <div v-if="task.speed" class="fm-task-meta">
                <span>{{ formatSpeed(task.speed) }}</span>
                <span v-if="task.fileSize">{{ formatBytes(task.fileSize) }}</span>
              </div>
              <div
                v-if="
                  task.status === 'uploading' ||
                  task.status === 'downloading' ||
                  task.status === 'paused'
                "
                class="fm-task-actions"
              >
                <button
                  v-if="task.status === 'paused' && task.direction !== 'download'"
                  type="button"
                  class="tiny-btn"
                  :title="t('upload.resume')"
                  @click="transferQueue.resume(task.id)"
                >
                  <Play :size="11" />
                </button>
                <button
                  v-if="task.status === 'uploading'"
                  type="button"
                  class="tiny-btn"
                  :title="t('upload.pause')"
                  @click="transferQueue.pause(task.id)"
                >
                  <PauseCircle :size="11" />
                </button>
                <button
                  type="button"
                  class="tiny-btn danger"
                  :title="t('upload.cancel')"
                  @click="transferQueue.cancel(task.id)"
                >
                  <XCircle :size="11" />
                </button>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </template>
    <div v-else class="fm-empty">
      <p>{{ t('dashboard.missingSession') }}</p>
    </div>
  </div>
</template>

<style scoped>
.file-manager-root {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background-color: #16161a;
  color: #e8e8e8;
  overflow: hidden;
  font-size: 12px;
}

.fm-disconnected {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 12px;
  color: #f87171;
  background: rgba(239, 68, 68, 0.12);
}
.fm-disconnected .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ef4444;
}

.fm-layout {
  flex: 1;
  min-height: 0;
  display: flex;
}

.fm-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.fm-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.fm-path {
  flex: 1;
  min-width: 0;
}
.fm-path-text {
  font-family: var(--font-mono);
  font-size: 11px;
  color: #9d9d9d;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
}
.fm-toolbar-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.fm-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  background: transparent;
  color: #aaa;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.fm-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #fff;
}
.fm-btn.primary {
  border-color: rgba(250, 204, 21, 0.3);
  color: #ffd54a;
}

.fm-browser {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 8px 12px;
}

.fm-transfers {
  width: 260px;
  flex-shrink: 0;
  border-left: 1px solid rgba(255, 255, 255, 0.06);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.fm-transfers-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.fm-transfers-title {
  font-weight: 600;
  font-size: 12px;
  color: #bbb;
}
.fm-transfers-actions {
  display: flex;
  gap: 4px;
}
.icon-btn-sm {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #777;
  cursor: pointer;
}
.icon-btn-sm:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #fff;
}
.fm-transfers-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
}
.fm-transfers-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: #555;
}
.fm-transfers-empty p {
  margin: 0;
  font-size: 11px;
}
.fm-task {
  padding: 8px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 8px;
}
.fm-task.success {
  border-color: rgba(16, 185, 129, 0.35);
}
.fm-task.error {
  border-color: rgba(239, 68, 68, 0.35);
}
.fm-task.cancelled {
  opacity: 0.7;
}
.fm-task-header {
  display: flex;
  align-items: center;
  gap: 6px;
}
.fm-task-icon {
  flex-shrink: 0;
}
.fm-task-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  font-weight: 600;
  color: #ececec;
}
.fm-task-time {
  font-size: 10px;
  color: #888;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}
.fm-task-progress {
  height: 4px;
  border-radius: 2px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.4);
  margin-top: 6px;
}
.fm-task-progress-fill {
  height: 100%;
  border-radius: 2px;
}
.fill-uploading {
  background: #3b82f6;
}
.fill-downloading {
  background: #8b5cf6;
}
.fill-paused {
  background: #facc15;
}
.fill-success {
  background: #10b981;
}
.fill-error {
  background: #ef4444;
}
.fm-task-meta {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin-top: 4px;
  font-size: 10px;
  color: #777;
}
.fm-task-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}
.tiny-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 22px;
  padding: 0 6px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.04);
  color: #bbb;
  cursor: pointer;
}
.tiny-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.09);
}
.tiny-btn.danger:hover {
  color: #f87171;
  background: rgba(239, 68, 68, 0.1);
}

.fm-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #6b7280;
  font-size: 13px;
}
.fm-empty p {
  margin: 0;
}
</style>
