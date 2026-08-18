<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Folder,
  File,
  FileSymlink,
  Download,
  Trash2,
  Pencil,
  X,
  Check,
  ArrowUp,
  ArrowDown,
} from 'lucide-vue-next';
import { useSftp, normalizePath } from '@/composables/use-sftp';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import type { SftpEntry } from '@/core/types';
import { formatBytes } from '@/core/types';

const { t } = useI18n();

interface Props {
  sessionId: string;
  /** Optional path to open on mount (e.g. the terminal's current directory). */
  initialPath?: string;
  /** Bumping this value triggers a refresh of the current directory. */
  refreshKey?: number;
  /** Compact single-line row rendering (hides extra columns & the header). */
  compact?: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  download: [entry: SftpEntry];
  /** Fires whenever the browser navigates to a new directory. */
  'current-path': [path: string];
}>();

const sessionIdRef = ref(props.sessionId);
const sftp = useSftp(sessionIdRef);

// --- Column sorting -----------------------------------------------------
type SortKey = 'name' | 'size' | 'mtime';
const sortKey = ref<SortKey>('name');
const sortAsc = ref(true);

const directionIcon = computed(() => (sortAsc.value ? ArrowUp : ArrowDown));

const toggleSort = (key: SortKey) => {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value;
  } else {
    sortKey.value = key;
    // Finder-like defaults: name ascending, size & date newest/largest first.
    sortAsc.value = key !== 'size' && key !== 'mtime';
  }
};

const sortedEntries = computed<SftpEntry[]>(() => {
  const key = sortKey.value;
  const asc = sortAsc.value;
  const cmp = (a: SftpEntry, b: SftpEntry): number => {
    if (key === 'size') {
      const r = a.size - b.size;
      return asc ? r : -r;
    }
    if (key === 'mtime') {
      const aa = a.mtime ?? 0;
      const bb = b.mtime ?? 0;
      return asc ? aa - bb : bb - aa;
    }
    const r = a.name.localeCompare(b.name, undefined, { numeric: true });
    return asc ? r : -r;
  };
  const dirs = sftp.entries.value.filter(e => e.isDir).sort(cmp);
  const files = sftp.entries.value.filter(e => !e.isDir).sort(cmp);
  return [...dirs, ...files];
});

// --- Selection ----------------------------------------------------------
const selectedPath = ref<string | null>(null);

const selectEntry = (entry: SftpEntry) => {
  selectedPath.value = entry.path;
};

// --- Inline "new directory" input ---------------------------------------
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

// --- Input sanitization ------------------------------------------------
// Reject names that would escape the current directory (`..`), create nested
// paths (`/` / `\`), or that are empty. Windows virtual drive handling
// (`/C:/`) is unaffected: this validates the *name* the user types for a new
// folder / rename, never the path prefix.
const validateEntryName = (name: string): string | null => {
  const trimmed = name.trim();
  if (!trimmed) return t('sftp.errorNameEmpty');
  if (trimmed === '.' || trimmed === '..') return t('sftp.errorNameDot');
  if (trimmed.includes('/') || trimmed.includes('\\')) {
    return t('sftp.errorNameSeparator');
  }
  return null;
};

const enterPath = async (entry: SftpEntry) => {
  // Symlinks can point at directories even when `isDir` is false, so allow
  // navigating into them; report a navigation error when it fails.
  if (!entry.isDir && !entry.isSymlink) return;
  actionError.value = '';
  const ok = await sftp.navigate(entry.path);
  if (ok) {
    selectedPath.value = null;
    actionError.value = '';
  } else {
    actionError.value = sftp.error.value;
  }
};

const onMkdir = async () => {
  // Clear any stale error banner at the start of the action.
  actionError.value = '';
  const name = mkdirName.value.trim();
  if (!name) {
    showMkdir.value = false;
    return;
  }
  const validationError = validateEntryName(name);
  if (validationError) {
    actionError.value = validationError;
    return;
  }
  const ok = await sftp.mkdir(name);
  if (!ok) actionError.value = sftp.error.value;
  else actionError.value = '';
  mkdirName.value = '';
  showMkdir.value = false;
};

const onRemove = (entry: SftpEntry) => {
  // Open the styled confirmation dialog (shown below) instead of the native
  // synchronous window.confirm, keeping the delete flow consistent with the
  // app's dark theme.
  entryToDelete.value = entry;
};

const entryToDelete = ref<SftpEntry | null>(null);

const confirmDeleteVisible = computed(() => entryToDelete.value !== null);

const confirmDeleteTitle = computed(() =>
  entryToDelete.value
    ? entryToDelete.value.isDir
      ? t('sftp.deleteDirConfirm', { name: entryToDelete.value.name })
      : t('sftp.deleteFileConfirm', { name: entryToDelete.value.name })
    : ''
);

const cancelDelete = () => {
  entryToDelete.value = null;
};

const confirmDelete = async () => {
  const entry = entryToDelete.value;
  entryToDelete.value = null;
  if (!entry) return;
  actionError.value = '';
  const ok = await sftp.remove(entry);
  if (!ok) actionError.value = sftp.error.value;
  else actionError.value = '';
};

const startRename = (entry: SftpEntry) => {
  editingPath.value = entry.path;
  editingName.value = entry.name;
};

const onRename = async () => {
  if (!editingPath.value) return;
  // Clear any stale error banner at the start of the action.
  actionError.value = '';
  const entry = sftp.entries.value.find(e => e.path === editingPath.value);
  if (!entry) {
    editingPath.value = null;
    return;
  }
  const newName = editingName.value.trim();
  const validationError = validateEntryName(newName);
  if (validationError) {
    actionError.value = validationError;
    return;
  }
  const ok = await sftp.rename(entry, newName);
  if (!ok) actionError.value = sftp.error.value;
  else actionError.value = '';
  editingPath.value = null;
};

const cancelRename = () => {
  editingPath.value = null;
  editingName.value = '';
};

onMounted(() => {
  // Open the requested initial path (e.g. the terminal's CWD), falling back
  // to the root when none is provided.
  void sftp.go(normalizePath(props.initialPath || '/'));
});

onUnmounted(() => {
  void sftp.dispose();
});

// Surface navigation to the parent so e.g. the file-manager window can target
// uploads at the current directory and keep its own state in sync.
watch(
  () => sftp.currentPath.value,
  path => {
    emit('current-path', path);
  },
  { immediate: true }
);

// Reload the current directory when the parent bumps `refreshKey` (e.g. after
// finishing an upload into this directory).
watch(
  () => props.refreshKey,
  () => {
    void sftp.refresh();
  }
);

// The address bar in the parent window drives navigation. Expose a small,
// typed imperative API so the toolbar can call us without duplicating the
// SFTP state outside this component.
defineExpose({
  /** Navigate directly to an absolute path (no history push). */
  go: sftp.go,
  goUp: sftp.goUp,
  goHome: sftp.goHome,
  refresh: sftp.refresh,
  /** Open the inline "new folder" input. */
  newFolder: () => {
    showMkdir.value = true;
  },
  /** Whether a directory listing is currently in flight (blocks uploads). */
  loading: sftp.loading,
  /** Whether an entry with this name exists in the current directory. */
  exists: (name: string) => sftp.entries.value.some(e => e.name === name),
});
</script>

<template>
  <div class="sftp-browser" :class="{ compact: props.compact }">
    <!-- Inline "new folder" input, requested from the parent toolbar -->
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
      <button type="button" class="tiny-btn ok" :title="t('sftp.newDir')" @click="onMkdir">
        <Check :size="12" />
      </button>
      <button
        type="button"
        class="tiny-btn"
        :title="t('common.cancel')"
        @click="showMkdir = false"
      >
        <X :size="12" />
      </button>
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

    <!-- Sortable column header (detail mode only) -->
    <div v-if="!props.compact" class="column-header">
      <button
        type="button"
        class="th name"
        :class="{ active: sortKey === 'name' }"
        :title="t('sftp.name')"
        @click="toggleSort('name')"
      >
        <span>{{ t('sftp.name') }}</span>
        <component :is="directionIcon" v-if="sortKey === 'name'" :size="12" />
      </button>
      <button
        type="button"
        class="th size"
        :class="{ active: sortKey === 'size' }"
        :title="t('sftp.size')"
        @click="toggleSort('size')"
      >
        <span>{{ t('sftp.size') }}</span>
        <component :is="directionIcon" v-if="sortKey === 'size'" :size="12" />
      </button>
      <button
        type="button"
        class="th mtime"
        :class="{ active: sortKey === 'mtime' }"
        :title="t('sftp.modified')"
        @click="toggleSort('mtime')"
      >
        <span>{{ t('sftp.modified') }}</span>
        <component :is="directionIcon" v-if="sortKey === 'mtime'" :size="12" />
      </button>
      <span class="th spacer" aria-hidden="true" />
    </div>

    <div
      v-if="!sftp.loading.value && sortedEntries.length === 0"
      class="sftp-state empty"
    >
      <div class="empty-icon">
        <Folder :size="30" />
      </div>
      <p>{{ t('sftp.emptyDir') }}</p>
    </div>

    <div
      v-else
      class="file-list scrollbar-thin"
      :class="{ loading: sftp.loading.value }"
    >
      <div
        v-for="entry in sortedEntries"
        :key="entry.path"
        class="file-row"
        :class="{ dir: entry.isDir, selected: selectedPath === entry.path }"
        role="row"
        @click="selectEntry(entry)"
        @dblclick="enterPath(entry)"
      >
        <div class="file-icon-wrap">
          <Folder
            v-if="entry.isDir"
            :size="16"
            class="file-icon dir"
            fill="currentColor"
          />
          <FileSymlink
            v-else-if="entry.isSymlink"
            :size="16"
            class="file-icon symlink"
          />
          <File v-else :size="16" class="file-icon file" />
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
            <Check :size="12" />
          </button>
          <button type="button" class="tiny-btn" @click="cancelRename">
            <X :size="12" />
          </button>
        </template>

        <template v-else>
          <span class="file-name" :title="entry.name">{{ entry.name }}</span>
          <span class="file-size" :class="{ 'is-dir': entry.isDir }">
            {{ entry.isDir ? '—' : formatBytes(entry.size) }}
          </span>
          <span class="file-mtime">{{ formatMtime(entry.mtime) }}</span>
          <div class="file-actions" role="group">
            <button
              v-if="!entry.isDir"
              type="button"
              class="action-btn"
              :title="t('sftp.download')"
              @click.stop="emit('download', entry)"
            >
              <Download :size="13" />
            </button>
            <button
              type="button"
              class="action-btn"
              :title="t('sftp.rename')"
              @click.stop="startRename(entry)"
            >
              <Pencil :size="13" />
            </button>
            <button
              type="button"
              class="action-btn danger"
              :title="t('sftp.delete')"
              @click.stop="onRemove(entry)"
            >
              <Trash2 :size="13" />
            </button>
          </div>
        </template>
      </div>
    </div>

    <!-- Footer status bar -->
    <div class="fm-footer">
      <span>{{ t('sftp.items', { count: sftp.entries.value.length }) }}</span>
    </div>

    <!-- Styled delete confirmation dialog -->
    <ConfirmDialog
      :visible="confirmDeleteVisible"
      :title="t('sftp.delete')"
      :message="confirmDeleteTitle"
      :confirm-text="t('sftp.delete')"
      :cancel-text="t('upload.cancel')"
      :is-danger="true"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </div>
</template>

<style scoped>
.sftp-browser {
  display: flex;
  flex-direction: column;
  flex: 1 1 0;
  min-height: 0;
  height: auto;
  overflow: hidden;
  gap: 4px;
  font-size: 13px;
}

/* Layout for the detail/view area: header + list + footer all stretch. */
.column-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
  padding: 4px 12px;
  border-bottom: 1px solid var(--color-border-secondary);
  color: var(--color-text-tertiary);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  user-select: none;
}
.sftp-browser.compact .column-header {
  display: none;
}

.th {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  letter-spacing: inherit;
  text-transform: inherit;
  cursor: pointer;
  border-radius: var(--radius-xs);
  padding: 2px 4px;
  transition: color var(--transition-fast), background var(--transition-fast);
}
.th:hover {
  color: var(--color-text-primary);
  background: var(--color-interactive-hover);
}
.th.active {
  color: var(--color-text-primary);
  font-weight: 600;
}
.th.name {
  flex: 1;
  min-width: 0;
  justify-content: flex-start;
}
.th.size {
  flex: 0 0 64px;
  justify-content: flex-end;
}
.th.mtime {
  flex: 0 0 92px;
  justify-content: flex-end;
}
.th.spacer {
  flex: 0 0 78px;
}
.sftp-browser.compact .th.spacer {
  flex: 0 0 0;
}

.mkdir-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
}

.inline-input {
  width: 100%;
  max-width: 340px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  color: var(--color-text-primary);
  padding: 5px 8px;
  font-size: 13px;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}
.inline-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}
.inline-input.grow {
  flex: 1;
  max-width: none;
}

.tiny-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--color-interactive-hover);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}
.tiny-btn.ok {
  background: color-mix(in srgb, var(--color-primary) 18%, transparent);
  color: var(--color-primary);
}
.tiny-btn:hover {
  color: var(--color-text-primary);
  background: var(--color-interactive-active);
}

.sftp-error {
  color: var(--color-danger);
  font-size: 12px;
  padding: 6px 12px;
  word-break: break-word;
  background: color-mix(in srgb, var(--color-danger) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--color-danger) 25%, transparent);
  border-radius: var(--radius-sm);
  margin: 0 12px;
  flex: 0 0 auto;
}

.sftp-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--color-text-tertiary);
}
.sftp-state .empty-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  border-radius: var(--radius-xl);
  background: var(--color-bg-tertiary);
  color: var(--color-text-tertiary);
}
.sftp-state p {
  margin: 0;
  font-size: 12px;
}

.file-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 8px;
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
  background: var(--color-primary);
  opacity: 0;
  border-radius: 2px;
}
.list-progress.active {
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
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
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: default;
  position: relative;
  transition: background var(--transition-fast);
}
.file-row:hover {
  background: var(--color-interactive-hover);
}
.file-row.selected {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}
.file-row.dir {
  cursor: default;
}

.sftp-browser.compact .file-row {
  padding: 3px 10px;
}

.file-icon-wrap {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
}
.file-icon.dir {
  color: var(--color-primary);
}
.file-icon.symlink {
  color: var(--color-text-secondary);
}
.file-icon.file {
  color: var(--color-text-tertiary);
}

.file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-primary);
  font-size: 13px;
}

.file-size {
  flex: 0 0 64px;
  text-align: right;
  color: var(--color-text-tertiary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.file-size.is-dir {
  color: var(--color-text-placeholder);
}
.file-mtime {
  flex: 0 0 92px;
  text-align: right;
  color: var(--color-text-tertiary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.sftp-browser.compact .file-size,
.sftp-browser.compact .file-mtime {
  display: none;
}

.file-actions {
  display: inline-flex;
  align-items: center;
  gap: 0;
  flex: 0 0 72px;
  justify-content: flex-end;
  visibility: hidden;
  opacity: 0;
  transition: opacity var(--transition-fast), visibility var(--transition-fast);
}
.file-row:hover .file-actions,
.file-row.selected .file-actions,
.file-row:focus-within .file-actions {
  visibility: visible;
  opacity: 1;
}

.action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}
.action-btn:hover {
  background: var(--color-interactive-active);
  color: var(--color-text-primary);
}
.action-btn.danger:hover {
  background: color-mix(in srgb, var(--color-danger) 15%, transparent);
  color: var(--color-danger);
}

.fm-footer {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 4px 14px;
  border-top: 1px solid var(--color-border-secondary);
  color: var(--color-text-tertiary);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
</style>
