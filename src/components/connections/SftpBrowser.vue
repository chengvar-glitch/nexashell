<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Folder,
  File,
  FileSymlink,
  ArrowUp,
  RotateCw,
  Home,
  LocateFixed,
  Download,
  Trash2,
  Pencil,
  X,
  Check,
  ChevronRight,
  FolderPlus,
} from 'lucide-vue-next';
import { useSftp, parentOfPath, normalizePath } from '@/composables/use-sftp';
import type { SftpEntry } from '@/core/types';
import { formatBytes } from '@/core/types';

const { t } = useI18n();

interface Props {
  sessionId: string;
  /** Optional path to open on mount (e.g. the terminal's current directory). */
  initialPath?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  download: [entry: SftpEntry];
}>();

const sessionIdRef = ref(props.sessionId);
const sftp = useSftp(sessionIdRef);

const breadcrumbs = computed<Array<{ label: string; path: string }>>(() => {
  const cur = sftp.currentPath.value;
  if (cur === '/') return [{ label: '/', path: '/' }];
  const parts = cur.split('/').filter(Boolean);
  const crumbs: Array<{ label: string; path: string }> = [
    { label: '/', path: '/' },
  ];
  let acc = '';
  for (const part of parts) {
    acc += `/${part}`;
    crumbs.push({ label: part, path: acc });
  }
  return crumbs;
});

const sortedEntries = computed<SftpEntry[]>(() => {
  const dirs = sftp.entries.value.filter(e => e.isDir);
  const files = sftp.entries.value.filter(e => !e.isDir);
  const sortByName = (a: SftpEntry, b: SftpEntry) =>
    a.name.localeCompare(b.name);
  return [...dirs.sort(sortByName), ...files.sort(sortByName)];
});

// Inline "new directory" input
const showMkdir = ref(false);
const mkdirName = ref('');
// Inline "rename" editing per entry path
const editingPath = ref<string | null>(null);
const editingName = ref('');
const actionError = ref<string>('');

const formatMtime = (mtime?: number): string => {
  if (!mtime) return '';
  return new Date(mtime * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
};

const enterPath = async (entry: SftpEntry) => {
  if (!entry.isDir) return;
  await sftp.navigate(entry.path);
};

const onMkdir = async () => {
  const name = mkdirName.value.trim();
  if (!name) {
    showMkdir.value = false;
    return;
  }
  const ok = await sftp.mkdir(name);
  if (!ok) actionError.value = sftp.error.value;
  mkdirName.value = '';
  showMkdir.value = false;
};

const onRemove = async (entry: SftpEntry) => {
  const title = entry.isDir
    ? t('sftp.deleteDirConfirm', { name: entry.name })
    : t('sftp.deleteFileConfirm', { name: entry.name });
  if (!window.confirm(title)) return;
  const ok = await sftp.remove(entry);
  if (!ok) actionError.value = sftp.error.value;
};

const startRename = (entry: SftpEntry) => {
  editingPath.value = entry.path;
  editingName.value = entry.name;
};

const onRename = async () => {
  if (!editingPath.value) return;
  const entry = sftp.entries.value.find(e => e.path === editingPath.value);
  if (!entry) {
    editingPath.value = null;
    return;
  }
  const ok = await sftp.rename(entry, editingName.value.trim());
  if (!ok) actionError.value = sftp.error.value;
  editingPath.value = null;
};

const cancelRename = () => {
  editingPath.value = null;
  editingName.value = '';
};

/**
 * Jump the file list back to the terminal's current remote directory (the
 * `initialPath` prop, kept up to date by the parent). Disabled at root.
 */
const goToCurrentDir = async (): Promise<void> => {
  const target = normalizePath(props.initialPath || '/');
  await sftp.go(target);
};

onMounted(() => {
  // Open the requested initial path (e.g. the terminal's CWD), falling back
  // to the root when none is provided.
  void sftp.go(normalizePath(props.initialPath || '/'));
});

onUnmounted(() => {
  void sftp.dispose();
});
</script>

<template>
  <div class="sftp-browser">
    <div class="sftp-toolbar">
      <div class="sftp-nav">
        <button
          type="button"
          class="toolbar-btn"
          :title="t('sftp.up')"
          :disabled="parentOfPath(sftp.currentPath.value) === null"
          @click="sftp.goUp"
        >
          <ArrowUp :size="13" />
        </button>
        <button
          type="button"
          class="toolbar-btn"
          :title="t('sftp.goHome')"
          @click="sftp.goHome('/')"
        >
          <Home :size="13" />
        </button>
        <button
          type="button"
          class="toolbar-btn"
          :title="t('sftp.refresh')"
          :disabled="sftp.loading.value"
          @click="sftp.refresh"
        >
          <RotateCw :size="13" :class="{ spinning: sftp.loading.value }" />
        </button>
        <button
          type="button"
          class="toolbar-btn locate"
          :title="t('sftp.locateCurrentDir')"
          :disabled="sftp.loading.value"
          @click="goToCurrentDir"
        >
          <LocateFixed :size="13" />
        </button>
      </div>
      <button
        type="button"
        class="toolbar-btn add"
        :title="t('sftp.newDir')"
        @click="showMkdir = !showMkdir"
      >
        <FolderPlus :size="13" />
      </button>
    </div>

    <div v-if="showMkdir" class="mkdir-row">
      <input
        v-model="mkdirName"
        type="text"
        class="inline-input"
        :placeholder="t('sftp.dirNamePlaceholder')"
        autofocus
        @keydown.enter="onMkdir"
        @keydown.esc="showMkdir = false"
      />
      <button type="button" class="tiny-btn ok" @click="onMkdir">
        <Check :size="11" />
      </button>
      <button type="button" class="tiny-btn" @click="showMkdir = false">
        <X :size="11" />
      </button>
    </div>

    <!-- Breadcrumb -->
    <div class="breadcrumb">
      <template v-for="(crumb, idx) in breadcrumbs" :key="crumb.path">
        <button
          type="button"
          class="crumb"
          :class="{ current: idx === breadcrumbs.length - 1 }"
          @click="sftp.go(crumb.path)"
        >
          {{ crumb.label }}
        </button>
        <ChevronRight
          v-if="idx < breadcrumbs.length - 1"
          :size="10"
          class="crumb-sep"
        />
      </template>
    </div>

    <div v-if="actionError" class="sftp-error">{{ actionError }}</div>
    <div v-if="sftp.error.value && !actionError" class="sftp-error">
      {{ sftp.error.value }}
    </div>

    <!-- Thin indeterminate progress bar shown while a directory loads. Keeps
         the previous list visible so navigation doesn't blank-flicker. -->
    <div class="list-progress" :class="{ active: sftp.loading.value }">
      <div class="list-progress-bar" />
    </div>

    <div
      v-if="!sftp.loading.value && sortedEntries.length === 0"
      class="sftp-state empty"
    >
      <Folder :size="22" />
      <p>{{ t('sftp.emptyDir') }}</p>
    </div>

    <div
      v-else
      class="file-list"
      :class="{ loading: sftp.loading.value }"
    >
      <div
        v-for="entry in sortedEntries"
        :key="entry.path"
        class="file-row"
        :class="{ dir: entry.isDir }"
        @dblclick="enterPath(entry)"
      >
        <div class="file-icon-wrap" @click="enterPath(entry)">
          <Folder
            v-if="entry.isDir"
            :size="15"
            class="file-icon dir"
            fill="currentColor"
          />
          <FileSymlink
            v-else-if="entry.isSymlink"
            :size="15"
            class="file-icon symlink"
          />
          <File v-else :size="15" class="file-icon file" />
        </div>

        <!-- Rename inline editor -->
        <template v-if="editingPath === entry.path">
          <input
            v-model="editingName"
            type="text"
            class="inline-input grow"
            autofocus
            @keydown.enter="onRename"
            @keydown.esc="cancelRename"
          />
          <button type="button" class="tiny-btn ok" @click="onRename">
            <Check :size="11" />
          </button>
          <button type="button" class="tiny-btn" @click="cancelRename">
            <X :size="11" />
          </button>
        </template>

        <template v-else>
          <button
            type="button"
            class="file-name"
            :title="entry.name"
            @click="enterPath(entry)"
          >
            {{ entry.name }}
          </button>
          <span class="file-size">
            {{ entry.isDir ? '—' : formatBytes(entry.size) }}
          </span>
          <span class="file-mtime">{{ formatMtime(entry.mtime) }}</span>
          <div class="file-actions">
            <button
              v-if="!entry.isDir"
              type="button"
              class="action-btn"
              :title="t('sftp.download')"
              @click.stop="emit('download', entry)"
            >
              <Download :size="12" />
            </button>
            <button
              type="button"
              class="action-btn"
              :title="t('sftp.rename')"
              @click.stop="startRename(entry)"
            >
              <Pencil :size="12" />
            </button>
            <button
              type="button"
              class="action-btn danger"
              :title="t('sftp.delete')"
              @click.stop="onRemove(entry)"
            >
              <Trash2 :size="12" />
            </button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sftp-browser {
  display: flex;
  flex-direction: column;
  /* Grow to fill the panel without depending on a definite % height: the
     flex chain (dashboard .accordion-content → this root) is bounded, and
     using flex-basis 0 + min-height:0 lets .file-list scroll internally. */
  flex: 1 1 0;
  min-height: 0;
  height: auto;
  overflow: hidden;
  gap: 6px;
  font-size: 12px;
}

.sftp-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.sftp-nav {
  display: flex;
  gap: 4px;
}

.toolbar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #9d9d9d;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.toolbar-btn:hover:not(:disabled) {
  background: #3c3c3c;
  color: #fff;
}
.toolbar-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.toolbar-btn.add {
  color: #facc15;
}
.toolbar-btn.add:hover {
  background: rgba(250, 204, 21, 0.15);
}
.toolbar-btn.locate {
  color: #60a5fa;
}
.toolbar-btn.locate:hover:not(:disabled) {
  background: rgba(96, 165, 250, 0.15);
  color: #60a5fa;
}

.mkdir-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.breadcrumb {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 2px;
  padding: 4px 6px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 4px;
  min-height: 24px;
}
.crumb {
  border: none;
  background: transparent;
  color: #9d9d9d;
  font-size: 11px;
  padding: 1px 3px;
  border-radius: 3px;
  cursor: pointer;
}
.crumb:hover {
  color: #fff;
  background: #3a3a3a;
}
.crumb.current {
  color: #facc15;
  cursor: default;
}
.crumb-sep {
  color: #5a5a5a;
}

.inline-input {
  flex: 1;
  min-width: 0;
  background: #1e1e1e;
  border: 1px solid #3c3c3c;
  border-radius: 4px;
  color: #e8e8e8;
  padding: 4px 6px;
  font-size: 12px;
}
.inline-input:focus {
  outline: none;
  border-color: #facc15;
}
.inline-input.grow {
  flex: 1;
}

.tiny-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 4px;
  background: #3a3a3a;
  color: #9d9d9d;
  cursor: pointer;
}
.tiny-btn.ok {
  background: rgba(250, 204, 21, 0.15);
  color: #facc15;
}
.tiny-btn:hover {
  color: #fff;
}

.sftp-error {
  color: #ef4444;
  font-size: 11px;
  padding: 2px 4px;
  word-break: break-word;
}

.sftp-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: #6b7280;
  padding: 24px 0;
}
.sftp-state p {
  margin: 0;
}
.spinning {
  animation: spin 0.9s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.file-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1px;
  transition: opacity 0.15s ease;
}
.file-list.loading {
  opacity: 0.55;
  pointer-events: none;
}

/* Indeterminate progress bar shown above the list while loading. */
.list-progress {
  height: 2px;
  border-radius: 2px;
  overflow: hidden;
  background: transparent;
  position: relative;
  flex: 0 0 auto;
}
.list-progress-bar {
  position: absolute;
  inset: 0;
  width: 0;
  background: #facc15;
  opacity: 0;
  border-radius: 2px;
}
.list-progress.active {
  background: rgba(250, 204, 21, 0.12);
}
.list-progress.active .list-progress-bar {
  opacity: 1;
  width: 40%;
  animation: progress-indeterminate 1.1s ease-in-out infinite;
}
@keyframes progress-indeterminate {
  0% {
    left: -40%;
  }
  100% {
    left: 100%;
  }
}

.file-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 4px;
  border-radius: 4px;
  cursor: default;
}
.file-row:hover {
  background: rgba(255, 255, 255, 0.05);
}
.file-row.dir {
  cursor: pointer;
}

.file-icon-wrap {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
}
.file-icon.dir {
  color: #facc15;
}
.file-icon.symlink {
  color: #60a5fa;
}
.file-icon.file {
  color: #9d9d9d;
}

.file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
  border: none;
  background: transparent;
  color: #e8e8e8;
  font-size: 12px;
  padding: 0;
  cursor: inherit;
}

.file-size {
  flex: 0 0 52px;
  text-align: right;
  color: #6b7280;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.file-mtime {
  flex: 0 0 70px;
  text-align: right;
  color: #6b7280;
  font-size: 10px;
}

.file-actions {
  display: none;
  align-items: center;
  gap: 2px;
}
.file-row:hover .file-actions,
.file-row:focus-within .file-actions {
  display: inline-flex;
}

.action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 3px;
  background: transparent;
  color: #9d9d9d;
  cursor: pointer;
}
.action-btn:hover {
  background: #3c3c3c;
  color: #fff;
}
.action-btn.danger:hover {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}
</style>
