<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
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
  FolderUp,
  FolderPlus,
  Home,
  RotateCw,
  LayoutList,
  PanelRightClose,
  CircleX,
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

// Reference to the embedded browser so the unified toolbar can drive
// navigation/mkdir through the imperative API exposed by SftpBrowser.
const browserRef = ref<InstanceType<typeof SftpBrowser> | null>(null);

// View mode: "detail" (sortable columns) vs "compact" (single-line rows).
const compactView = ref(false);

// Editable address bar.
const addressDraft = ref('');
const addressEditing = ref(false);
const addressFocused = ref(false);

// Local drag-and-drop overlay state via Tauri native events.
const isDragging = ref(false);
let unlistenDrop: UnlistenFn | null = null;
let unlistenDragEnter: UnlistenFn | null = null;
let unlistenDragLeave: UnlistenFn | null = null;

const activeTaskCount = computed(
  () =>
    transferQueue.tasks.value.filter(
      t => t.status === 'uploading' || t.status === 'downloading' || t.status === 'paused'
    ).length
);

onMounted(async () => {
  if (!sessionId.value) return;
  disconnectUnlisten = await listen(`ssh-disconnected-${sessionId.value}`, () => {
    disconnected.value = true;
  });
  await transferQueue.setupListeners();

  // Tauri native drag events. Dropping picks up the absolute local paths and
  // uploads them into the current remote directory.
  unlistenDrop = await listen<{ paths: string[] }>(
    'tauri://drag-drop',
    event => {
      isDragging.value = false;
      void uploadLocalPaths(event.payload.paths);
    }
  );
  unlistenDragEnter = await listen('tauri://drag-enter', () => {
    isDragging.value = true;
  });
  unlistenDragLeave = await listen('tauri://drag-leave', () => {
    isDragging.value = false;
  });
});

onUnmounted(() => {
  if (disconnectUnlisten) {
    void disconnectUnlisten();
    disconnectUnlisten = null;
  }
  if (unlistenDrop) {
    void unlistenDrop();
    unlistenDrop = null;
  }
  if (unlistenDragEnter) {
    void unlistenDragEnter();
    unlistenDragEnter = null;
  }
  if (unlistenDragLeave) {
    void unlistenDragLeave();
    unlistenDragLeave = null;
  }
  transferQueue.dispose();
});

const onCurrentPath = (path: string) => {
  currentRemotePath.value = path;
};

const refreshBrowser = () => {
  refreshKey.value += 1;
};

const goUp = () => void browserRef.value?.goUp();
const goHome = () => void browserRef.value?.goHome('/');
const goRefresh = () => void browserRef.value?.refresh();
const newFolder = () => void browserRef.value?.newFolder();

/** Start an upload for a list of local file paths into the current remote dir. */
const uploadLocalPaths = async (paths: string[]) => {
  if (!sessionId.value || paths.length === 0) return;
  const base = currentRemotePath.value === '/' ? '' : currentRemotePath.value;
  for (const localPath of paths) {
    const fileName = localPath.split('/').pop() || localPath;
    await transferQueue.startUpload(localPath, `${base}/${fileName}`, fileName);
  }
  refreshBrowser();
  showTransfers.value = true;
};

/** Pick one or more local files and upload them into the current remote dir. */
const onUpload = async () => {
  if (!sessionId.value) return;
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({ multiple: true });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    await uploadLocalPaths(paths);
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

// --- Address bar ------------------------------------------------------
const addressText = computed(() =>
  addressEditing.value ? addressDraft.value : currentRemotePath.value
);

const startAddressEdit = () => {
  addressDraft.value = currentRemotePath.value;
  addressEditing.value = true;
};

const onAddressInput = (e: Event) => {
  addressDraft.value = (e.target as HTMLInputElement).value;
  addressEditing.value = true;
};

const commitAddress = () => {
  if (!addressEditing.value) return;
  const target = addressDraft.value.trim();
  addressEditing.value = false;
  if (!target || target === currentRemotePath.value) return;
  void browserRef.value?.go(target);
};

const cancelAddress = () => {
  addressEditing.value = false;
  addressDraft.value = '';
};

// --- Transfer panel helpers -------------------------------------------
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

const taskStatusClass = (task: UploadTask): string => {
  switch (task.status) {
    case 'success':
      return 'status-success';
    case 'error':
      return 'status-error';
    case 'cancelled':
      return 'status-cancelled';
    case 'paused':
      return 'status-paused';
    case 'downloading':
      return 'status-download';
    case 'uploading':
      return 'status-upload';
    default:
      return 'status-active';
  }
};
</script>

<template>
  <div class="file-manager-root">
    <template v-if="sessionId">
      <div v-if="disconnected" class="fm-disconnected">
        <span class="dot" />
        {{ t('dashboard.disconnected') }}
        <button type="button" class="retry-close" @click="disconnected = false">
          <CircleX :size="13" />
        </button>
      </div>

      <div v-else class="fm-layout">
        <div class="fm-main">
          <!-- Unified tool / address bar -->
          <div class="fm-toolbar">
            <div class="tb-nav">
              <button
                type="button"
                class="tb-btn"
                :title="t('sftp.up')"
                @click="goUp"
              >
                <ArrowUp :size="15" />
              </button>
              <button
                type="button"
                class="tb-btn"
                :title="t('sftp.goHome')"
                @click="goHome"
              >
                <Home :size="15" />
              </button>
              <button
                type="button"
                class="tb-btn"
                :title="t('sftp.refresh')"
                @click="goRefresh"
              >
                <RotateCw :size="15" />
              </button>
            </div>

            <div
              class="tb-address"
              :class="{ editing: addressEditing, focused: addressFocused }"
            >
              <FolderUp v-if="!addressEditing" :size="14" class="tb-address-icon" />
              <input
                :value="addressText"
                type="text"
                spellcheck="false"
                :placeholder="currentRemotePath"
                @input="onAddressInput"
                @focus="startAddressEdit(); addressFocused = true"
                @blur="addressFocused = false; commitAddress()"
                @keydown.enter="commitAddress()"
                @keydown.esc="cancelAddress()"
              />
            </div>

            <div class="tb-actions">
              <button
                type="button"
                class="tb-btn"
                :class="{ active: compactView }"
                :title="compactView ? t('sftp.detailView') : t('sftp.compactView')"
                @click="compactView = !compactView"
              >
                <LayoutList :size="15" />
              </button>
              <button
                type="button"
                class="tb-btn"
                :title="t('sftp.newDir')"
                @click="newFolder"
              >
                <FolderPlus :size="15" />
              </button>
              <button type="button" class="tb-btn upload" :title="t('dashboard.upload')" @click="onUpload">
                <Upload :size="15" />
                <span>{{ t('dashboard.upload') }}</span>
              </button>
              <button
                type="button"
                class="tb-btn transfers-toggle"
                :class="{ active: showTransfers }"
                :title="t('dashboard.transfersToggle')"
                @click="showTransfers = !showTransfers"
              >
                <Activity :size="15" />
                <span v-if="activeTaskCount" class="tb-badge">{{
                  activeTaskCount
                }}</span>
              </button>
            </div>
          </div>

          <div class="fm-browser">
            <SftpBrowser
              ref="browserRef"
              :session-id="sessionId"
              :refresh-key="refreshKey"
              :compact="compactView"
              @current-path="onCurrentPath"
              @download="downloadEntry"
            />
          </div>
        </div>

        <!-- Transfer queue panel (right side, collapsible) -->
        <aside class="fm-transfers" :class="{ collapsed: !showTransfers }">
          <div class="fm-transfers-inner">
            <div class="fm-transfers-header">
              <span class="fm-transfers-title">{{ t('dashboard.uploads') }}</span>
              <div class="fm-transfers-actions">
                <button
                  type="button"
                  class="icon-btn-sm"
                  :title="t('dashboard.clearAll')"
                  @click="transferQueue.clearCompleted()"
                >
                  <Trash2 :size="14" />
                </button>
                <button
                  type="button"
                  class="icon-btn-sm"
                  :title="t('dashboard.closePanel')"
                  @click="showTransfers = false"
                >
                  <PanelRightClose :size="14" />
                </button>
              </div>
            </div>

            <div
              v-if="transferQueue.tasks.value.length === 0"
              class="fm-transfers-empty"
            >
              <Activity :size="22" />
              <p>{{ t('dashboard.noUploadTasks') }}</p>
            </div>
            <div v-else class="fm-transfers-list scrollbar-thin">
              <div
                v-for="task in transferQueue.tasks.value"
                :key="task.id"
                class="fm-task"
                :class="taskStatusClass(task)"
              >
                <div class="fm-task-header">
                  <div class="fm-task-icon-wrap" :class="taskStatusClass(task)">
                    <component :is="taskStatusIcon(task.status)" :size="14" />
                  </div>
                  <span class="fm-task-name" :title="task.fileName || task.id">{{
                    task.fileName || 'Transfer'
                  }}</span>
                  <span class="fm-task-percent">{{
                    Math.floor(task.progress)
                  }}%</span>
                </div>
                <div class="fm-task-progress">
                  <div
                    class="fm-task-progress-fill"
                    :class="taskStatusClass(task)"
                    :style="{ width: `${Math.min(100, task.progress)}%` }"
                  />
                </div>
                <div v-if="task.speed || task.fileSize" class="fm-task-meta">
                  <span v-if="task.speed">{{ formatSpeed(task.speed) }}</span>
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
                    <Play :size="12" />
                  </button>
                  <button
                    v-if="task.status === 'uploading'"
                    type="button"
                    class="tiny-btn"
                    :title="t('upload.pause')"
                    @click="transferQueue.pause(task.id)"
                  >
                    <PauseCircle :size="12" />
                  </button>
                  <button
                    type="button"
                    class="tiny-btn danger"
                    :title="t('upload.cancel')"
                    @click="transferQueue.cancel(task.id)"
                  >
                    <XCircle :size="12" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </template>
    <div v-else class="fm-empty">
      <p>{{ t('dashboard.missingSession') }}</p>
    </div>

    <!-- Drag-and-drop upload overlay -->
    <Transition name="fm-fade">
      <div v-if="isDragging" class="fm-drop-overlay">
        <div class="fm-drop-card">
          <Upload :size="36" />
          <p>{{ t('ssh.dropFilesHere') }}</p>
          <span>
            {{ t('sftp.dropUpload', { path: currentRemotePath }) }}
          </span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.file-manager-root {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-primary);
  color: var(--color-text-primary);
  overflow: hidden;
  font-size: 13px;
}

.fm-disconnected {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px 14px;
  font-size: 12px;
  color: var(--color-danger);
  background: color-mix(in srgb, var(--color-danger) 12%, transparent);
}
.fm-disconnected .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-danger);
}
.fm-disconnected .retry-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: inherit;
  cursor: pointer;
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

/* ---- Unified toolbar ---- */
.fm-toolbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border-secondary);
  background-color: var(--color-bg-secondary);
}
.tb-nav,
.tb-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
}
.tb-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 8px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
  position: relative;
}
.tb-btn:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}
.tb-btn.active {
  background: var(--color-interactive-selected);
  color: var(--color-primary);
}
.tb-btn.upload {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  font-weight: 600;
}
.tb-btn.upload:hover {
  background: color-mix(in srgb, var(--color-primary) 20%, transparent);
  color: var(--color-primary);
}
.tb-badge {
  position: absolute;
  top: 2px;
  right: 2px;
  min-width: 14px;
  height: 14px;
  padding: 0 3px;
  border-radius: 7px;
  background: var(--color-primary);
  color: #fff;
  font-size: 9px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-variant-numeric: tabular-nums;
}

.tb-address {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 10px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background-color: var(--color-bg-elevated);
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}
.tb-address:hover {
  border-color: var(--color-border-primary);
}
.tb-address.focused,
.tb-address.editing {
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}
.tb-address-icon {
  flex: 0 0 auto;
  color: var(--color-text-tertiary);
}
.tb-address input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--color-text-primary);
  font-family: var(--font-mono);
  font-size: 12px;
}

/* ---- Browser area ---- */
.fm-browser {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* ---- Transfer panel ---- */
.fm-transfers {
  flex: 0 0 auto;
  width: 300px;
  min-width: 0;
  overflow: hidden;
  border-left: 1px solid var(--color-border-secondary);
  background-color: var(--color-bg-secondary);
  display: flex;
  flex-direction: column;
  transition: width var(--transition-base), border-color var(--transition-base);
}
.fm-transfers.collapsed {
  width: 0;
  border-left-width: 0;
}
.fm-transfers-inner {
  width: 300px;
  min-width: 300px;
  height: 100%;
  display: flex;
  flex-direction: column;
}
.fm-transfers-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border-secondary);
}
.fm-transfers-title {
  font-weight: 600;
  font-size: 13px;
  color: var(--color-text-primary);
}
.fm-transfers-actions {
  display: flex;
  gap: 4px;
}
.icon-btn-sm {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}
.icon-btn-sm:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
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
  gap: 10px;
  color: var(--color-text-placeholder);
}
.fm-transfers-empty p {
  margin: 0;
  font-size: 12px;
}

.fm-task {
  padding: 10px;
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}
.fm-task.status-upload {
  border-left: 3px solid var(--color-primary);
}
.fm-task.status-download {
  border-left: 3px solid var(--color-accent);
}
.fm-task.status-success {
  border-left: 3px solid #30d158;
}
.fm-task.status-error {
  border-left: 3px solid var(--color-danger);
}
.fm-task.status-paused {
  border-left: 3px solid #ffd60a;
}
.fm-task.status-cancelled {
  opacity: 0.6;
}

.fm-task-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.fm-task-icon-wrap {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  flex: 0 0 auto;
}
.fm-task-icon-wrap.status-upload {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 14%, transparent);
}
.fm-task-icon-wrap.status-download {
  color: var(--color-accent);
  background: color-mix(in srgb, var(--color-accent) 14%, transparent);
}
.fm-task-icon-wrap.status-success {
  color: #30d158;
  background: color-mix(in srgb, #30d158 14%, transparent);
}
.fm-task-icon-wrap.status-error {
  color: var(--color-danger);
  background: color-mix(in srgb, var(--color-danger) 14%, transparent);
}
.fm-task-icon-wrap.status-paused {
  color: #ffd60a;
  background: color-mix(in srgb, #ffd60a 14%, transparent);
}
.fm-task-icon-wrap.status-cancelled {
  color: var(--color-text-tertiary);
  background: var(--color-interactive-hover);
}
.fm-task-icon-wrap.status-active {
  color: var(--color-text-secondary);
  background: var(--color-interactive-hover);
}

.fm-task-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-primary);
}
.fm-task-percent {
  font-size: 11px;
  color: var(--color-text-secondary);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}
.fm-task-progress {
  height: 5px;
  border-radius: 3px;
  overflow: hidden;
  background: var(--color-interactive-hover);
  margin-top: 8px;
}
.fm-task-progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.2s var(--ease-snappy);
}
.fm-task-progress-fill.status-upload {
  background: var(--color-primary);
}
.fm-task-progress-fill.status-download {
  background: var(--color-accent);
}
.fm-task-progress-fill.status-success {
  background: #30d158;
}
.fm-task-progress-fill.status-error {
  background: var(--color-danger);
}
.fm-task-progress-fill.status-paused {
  background: #ffd60a;
}
.fm-task-progress-fill.status-active {
  background: var(--color-text-tertiary);
}
.fm-task-meta {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin-top: 4px;
  font-size: 11px;
  color: var(--color-text-tertiary);
  font-variant-numeric: tabular-nums;
}
.fm-task-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  margin-top: 8px;
}
.tiny-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 26px;
  height: 24px;
  padding: 0 8px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}
.tiny-btn:hover {
  color: var(--color-text-primary);
  background: var(--color-interactive-hover);
}
.tiny-btn.danger:hover {
  color: var(--color-danger);
  background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  border-color: color-mix(in srgb, var(--color-danger) 30%, transparent);
}

.fm-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary);
  font-size: 13px;
}
.fm-empty p {
  margin: 0;
}

/* ---- Drag-and-drop overlay ---- */
.fm-drop-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  backdrop-filter: blur(2px);
  pointer-events: auto;
}
.fm-drop-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px 56px;
  border: 1px dashed var(--color-primary);
  border-radius: var(--radius-2xl);
  background: var(--color-bg-elevated);
  color: var(--color-text-primary);
  box-shadow: var(--shadow-xl);
  text-align: center;
}
.fm-drop-card svg {
  color: var(--color-primary);
}
.fm-drop-card p {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}
.fm-drop-card span {
  font-size: 12px;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}

.fm-fade-enter-active,
.fm-fade-leave-active {
  transition: opacity 0.15s var(--ease-snappy);
}
.fm-fade-enter-from,
.fm-fade-leave-to {
  opacity: 0;
}
</style>
