<template>
  <div class="nexashell-home">
    <!-- Sidebar Navigation: Groups -->
    <aside class="home-sidebar">
      <div class="sidebar-section">
        <h4 class="section-title">
          {{ $t('home.views') }}
        </h4>
        <nav class="sidebar-nav">
          <button
            class="nav-item"
            :class="{ active: activeView === 'all' }"
            @click="setActiveView('all')"
          >
            <Home :size="16" />
            <span>{{ $t('home.allSessions') }}</span>
            <span class="count">{{ sessions.length }}</span>
          </button>
          <button
            class="nav-item"
            :class="{ active: activeView === 'favorites' }"
            @click="setActiveView('favorites')"
          >
            <Star :size="16" />
            <span>{{ $t('home.favorites') }}</span>
            <span v-if="favoriteCount > 0" class="count">{{
              favoriteCount
            }}</span>
          </button>
          <button
            class="nav-item"
            :class="{ active: activeView === 'recent' }"
            @click="setActiveView('recent')"
          >
            <History :size="16" />
            <span>{{ $t('home.recent') }}</span>
          </button>
        </nav>
      </div>

      <div class="sidebar-section">
        <div class="section-header">
          <h4 class="section-title">
            {{ $t('home.groups') }}
          </h4>
          <button
            class="add-btn"
            @click="showAddGroupInput = !showAddGroupInput"
          >
            <Plus :size="14" />
          </button>
        </div>
        <div v-if="showAddGroupInput" class="add-input-wrapper">
          <input
            v-model="newGroupName"
            type="text"
            class="add-input"
            placeholder="Group name..."
            autofocus
            @blur="handleGroupInputBlur"
            @keydown="handleGroupInputKeydown"
          />
          <div class="add-actions">
            <button
              class="action-btn tiny confirm"
              :title="t('common.confirm')"
              @mousedown.prevent
              @click="handleAddGroup"
            >
              <Check :size="12" />
            </button>
            <button
              class="action-btn tiny cancel"
              :title="t('common.cancel')"
              @mousedown.prevent
              @click="handleGroupInputBlur"
            >
              <X :size="12" />
            </button>
          </div>
        </div>
        <nav class="sidebar-nav">
          <div v-for="group in groups" :key="group.id" class="nav-item-wrapper">
            <template v-if="editingGroupId === group.id">
              <input
                v-model="renamingGroupName"
                type="text"
                class="rename-input"
                autofocus
                @blur="handleSaveGroupRename"
                @keydown.enter="handleSaveGroupRename"
                @keydown.esc="editingGroupId = null"
              />
            </template>
            <template v-else>
              <button
                class="nav-item"
                :class="{
                  active:
                    activeView === 'group' && selectedGroupId === group.id,
                }"
                @click="setActiveGroup(group.id)"
              >
                <component
                  :is="
                    activeView === 'group' && selectedGroupId === group.id
                      ? FolderOpen
                      : Folder
                  "
                  :size="16"
                />
                <span>{{ group.name }}</span>
                <span v-if="groupCounts[group.name] > 0" class="count">{{
                  groupCounts[group.name]
                }}</span>
              </button>
              <div class="item-actions">
                <button
                  class="action-btn small"
                  @click.stop="startEditGroup(group)"
                >
                  <Pencil :size="12" />
                </button>
                <button
                  class="action-btn small delete"
                  @click.stop="handleDeleteGroup(group.id)"
                >
                  <Minus :size="12" />
                </button>
              </div>
            </template>
          </div>
        </nav>
      </div>
    </aside>

    <!-- Main Content: Session Management -->
    <main class="home-main">
      <header class="main-header">
        <div class="title-area">
          <h3>{{ viewTitle }}</h3>
          <p class="subtitle">
            {{ $t('home.subtitle') }}
          </p>
        </div>
        <div class="header-actions">
          <button class="header-action-btn" @click="showImportDialog = true">
            <Import :size="14" />
            <span>{{ $t('home.import') }}</span>
          </button>
        </div>
      </header>

      <!-- Batch action bar: appears when sessions are selected -->
      <div v-if="selectedSessionIds.size > 0" class="batch-bar">
        <span class="batch-count">
          {{ t('home.batchSelected', { count: selectedSessionIds.size }) }}
        </span>
        <div class="batch-actions">
          <button class="batch-btn" @click="handleBatchFavorite">
            <Star
              :size="14"
              :fill="allSelectedAreFavorites ? 'currentColor' : 'none'"
            />
            {{
              allSelectedAreFavorites
                ? t('home.batchUnfavorite')
                : t('home.batchFavorite')
            }}
          </button>
          <button class="batch-btn" @click="toggleBatchGroupMenu($event)">
            <Folder :size="14" />
            {{ t('home.batchAddGroup') }}
          </button>
          <button class="batch-btn danger" @click="handleBatchDelete">
            <Trash2 :size="14" />
            {{ t('common.delete') }}
          </button>
          <button class="batch-btn" @click="clearSelection">
            <X :size="14" />
            {{ t('home.clearSelection') }}
          </button>
        </div>
      </div>

      <!-- Session Grid/Table Area -->
      <section class="session-manager">
        <!-- Empty/loading states, distinct from each other: loading (initial
             fetch), no sessions at all ("add first"), and a filtered view
             with no matching rows (no matches). -->
        <div v-if="isLoading" class="empty-state-container">
          <div class="empty-state-message">{{ $t('home.loading') }}</div>
        </div>

        <div
          v-else-if="sessions.length === 0"
          class="empty-state-container"
        >
          <button class="empty-card" @click="handleNewConnection">
            <Plus :size="32" class="plus" />
            <span>{{ $t('home.addFirst') }}</span>
          </button>
        </div>

        <div
          v-else-if="filteredSessions.length === 0"
          class="empty-state-container"
        >
          <div class="empty-state-message">{{ $t('home.noMatches') }}</div>
        </div>

        <div v-else class="session-table-wrapper">
          <table class="session-table">
            <thead>
              <tr>
                <th class="col-drag"></th>
                <th class="col-checkbox">
                  <input
                    type="checkbox"
                    class="session-checkbox"
                    :checked="allSelected"
                    :indeterminate="someSelected"
                    @change="toggleSelectAllGlobal($event)"
                  />
                </th>
                <th class="col-favorite"></th>
                <th class="col-pin"></th>
                <th class="col-name">{{ $t('home.serverName') }}</th>
                <th class="col-host">{{ $t('home.host') }}</th>
                <th class="col-groups-list">{{ $t('home.groups') }}</th>
                <th class="col-last-connect">{{ $t('home.lastConnected') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="session in filteredSessions"
                :key="session.id"
                class="session-row"
                :class="{ selected: isSessionSelected(session.id) }"
                @dblclick="handleQuickConnect(session)"
                @contextmenu.prevent="handleSessionContextMenu($event, session)"
              >
                <td class="col-drag">
                  <GripVertical :size="14" class="drag-handle" />
                </td>
                <td class="col-checkbox" @click.stop>
                  <input
                    type="checkbox"
                    class="session-checkbox"
                    :checked="isSessionSelected(session.id)"
                    @change="toggleSessionSelection(session.id)"
                  />
                </td>
                <td class="col-favorite">
                  <button
                    class="favorite-btn-small"
                    :class="{ active: session.is_favorite }"
                    @click.stop="toggleFavorite(session)"
                  >
                    <Star
                      :size="14"
                      :fill="session.is_favorite ? 'currentColor' : 'none'"
                    />
                  </button>
                </td>
                <td class="col-pin">
                  <button
                    class="pin-btn-small"
                    :class="{ active: session.is_pinned }"
                    :title="
                      session.is_pinned
                        ? t('home.unpin')
                        : t('home.pin')
                    "
                    @click.stop="togglePin(session)"
                  >
                    <component
                      :is="session.is_pinned ? Pin : PinOff"
                      :size="14"
                    />
                  </button>
                </td>
                <td class="col-name">
                  <div class="name-text">
                    {{ session.server_name }}
                  </div>
                </td>
                <td class="col-host">
                  <span class="host-text">
                    {{ session.username }}@{{ session.addr }}
                  </span>
                </td>
                <td class="col-groups-list">
                  <div class="table-groups">
                    <span
                      v-for="group in session.groups"
                      :key="group"
                      class="mini-group-tag"
                    >
                      {{ group }}
                    </span>
                    <span
                      v-if="!session.groups || session.groups.length === 0"
                      class="empty-text-dim"
                    >
                      -
                    </span>
                  </div>
                </td>
                <td class="col-last-connect">
                  <div class="time-actions-wrapper">
                    <span class="timestamp">
                      {{
                        session.last_connected_at
                          ? formatRelativeTime(
                              session.last_connected_at,
                              locale
                            )
                          : '-'
                      }}
                    </span>
                    <button
                      class="icon-action-btn more"
                      :title="t('common.more') || 'More'"
                      @click.stop="handleSessionContextMenu($event, session)"
                    >
                      <MoreVertical :size="16" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </main>

    <!-- Context Menu for Session Card -->
    <DropdownMenu
      :visible="contextMenuVisible"
      :items="contextMenuItems"
      :x="contextMenuX"
      :y="contextMenuY"
      @update:visible="contextMenuVisible = $event"
      @select="handleContextMenuSelect"
    />

    <!-- Batch group submenu -->
    <DropdownMenu
      v-model:visible="batchGroupMenuOpen"
      :items="batchGroupItems"
      :x="batchGroupX"
      :y="batchGroupY"
      @select="handleBatchGroupSelect"
    />

    <!-- Confirm Delete Dialog -->
    <ConfirmDialog
      :visible="showConfirmDialog"
      :title="confirmDialogTitle"
      :message="confirmDialogMessage"
      :confirm-text="$t('common.delete')"
      :cancel-text="$t('common.cancel')"
      :is-danger="true"
      @confirm="onConfirmDelete"
      @cancel="onCancelDelete"
    />

    <!-- Transient toast (copy feedback) -->
    <Transition name="toast">
      <div v-if="copiedMessage" class="home-toast" role="status">
        {{ copiedMessage }}
      </div>
    </Transition>

    <!-- XTerminal session import dialog -->
    <ImportXTerminalDialog
      :visible="showImportDialog"
      @update:visible="showImportDialog = $event"
      @imported="loadSessions"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, inject, onMounted, onUnmounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import {
  Home,
  Star,
  History,
  Folder,
  FolderOpen,
  Plus,
  Minus,
  Terminal,
  Copy,
  Pencil,
  Trash2,
  Check,
  X,
  GripVertical,
  MoreVertical,
  Import,
  Pin,
  PinOff,
  FolderTree,
  type LucideIcon,
} from 'lucide-vue-next';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import ImportXTerminalDialog from '@/components/home/ImportXTerminalDialog.vue';
import { openFileManagerWindow } from '@/features/window';
import { OPEN_SSH_FORM_KEY } from '@/core/types';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';
import { formatRelativeTime, parseDbTimestamp, sortSessions } from '@/core/utils/time-utils';
import { createLogger } from '@/core/utils/logger';
import { useToastMessage } from '@/composables';
import { sessionApi } from '@/features/session';
import type { SavedSessionDisplay } from '@/features/session/types';

const logger = createLogger('NexaShellHome');

interface Group {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

// Global states
const sessions = ref<SavedSessionDisplay[]>([]);
const groups = ref<Group[]>([]);
const isMounted = ref(false);
// True during the initial/refresh session fetch so the UI can show a
// distinct "loading" state instead of flashing the empty state.
const isLoading = ref(true);

// View and filter states
const activeView = ref<'all' | 'favorites' | 'recent' | 'group'>('all');
const selectedGroupId = ref<string | null>(null);
const selectedSessionIds = ref<Set<string>>(new Set());

const favoriteCount = computed(
  () => sessions.value.filter(s => s.is_favorite).length
);

// Group counts for sidebar
const countByNames = (
  extract: (s: SavedSessionDisplay) => string[] | undefined
) => {
  const counts: Record<string, number> = {};
  sessions.value.forEach(s => {
    (extract(s) || []).forEach(name => {
      counts[name] = (counts[name] || 0) + 1;
    });
  });
  return counts;
};

const groupCounts = computed(() => countByNames(s => s.groups));

// Input states for adding new groups
const showAddGroupInput = ref(false);
const newGroupName = ref('');

// Renaming states
const editingGroupId = ref<string | null>(null);
const renamingGroupName = ref('');

// Computed values for UI
const viewTitle = computed(() => {
  switch (activeView.value) {
    case 'favorites':
      return t('home.favorites');
    case 'recent':
      return t('home.recent');
    case 'group':
      return (
        groups.value.find(g => g.id === selectedGroupId.value)?.name ||
        t('common.group')
      );
    default:
      return t('home.allSessions');
  }
});

const filteredSessions = computed(() => {
  let result = [...sessions.value];

  // 1. Filter by view/category
  if (activeView.value === 'favorites') {
    result = result.filter(s => s.is_favorite);
  } else if (activeView.value === 'recent') {
    // Sort by last_connected_at descending and take only the first 10
    result = result
      .filter(s => s.last_connected_at)
      .sort((a, b) => {
        const dateA = parseDbTimestamp(a.last_connected_at);
        const dateB = parseDbTimestamp(b.last_connected_at);
        return dateB - dateA;
      })
      .slice(0, 10);
  } else if (activeView.value === 'group' && selectedGroupId.value) {
    const groupName = groups.value.find(
      g => g.id === selectedGroupId.value
    )?.name;
    if (groupName) {
      result = result.filter(s => s.groups?.includes(groupName));
    }
  }

  // Pinned sessions first (most recently pinned on top), then by recency.
  // The "recent" view keeps its own last-connected ordering.
  if (activeView.value !== 'recent') {
    result = sortSessions(result);
  }

  return result;
});

// Selection handlers
const setActiveView = (view: 'all' | 'favorites' | 'recent') => {
  activeView.value = view;
  selectedGroupId.value = null;
  selectedSessionIds.value = new Set();
};

const setActiveGroup = (groupId: string) => {
  activeView.value = 'group';
  selectedGroupId.value = groupId;
  selectedSessionIds.value = new Set();
};

const toggleFavorite = async (session: SavedSessionDisplay) => {
  try {
    const newStatus = !session.is_favorite;
    await sessionApi.toggleFavorite(session.id, newStatus);
    session.is_favorite = newStatus;
    logger.info('Toggled favorite', {
      session: session.server_name,
      isFavorite: newStatus,
    });
  } catch (error) {
    logger.error('Failed to toggle favorite', error);
  }
};

const togglePin = async (session: SavedSessionDisplay) => {
  try {
    const newStatus = !session.is_pinned;
    await sessionApi.togglePin(session.id, newStatus);
    session.is_pinned = newStatus;
    // Mirror the backend timestamp locally so the pinned-first sort orders a
    // freshly pinned row correctly until the next full reload.
    session.pinned_at = newStatus ? new Date().toISOString() : null;
    logger.info('Toggled pin', {
      session: session.server_name,
      isPinned: newStatus,
    });
  } catch (error) {
    logger.error('Failed to toggle pin', error);
  }
};

// Context menu states
const contextMenuVisible = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuItems = ref<
  Array<{
    key: string;
    label: string;
    danger?: boolean;
    divider?: boolean;
    icon?: LucideIcon;
    children?: Array<{
      key: string;
      label: string;
      danger?: boolean;
      disabled?: boolean;
      icon?: LucideIcon;
    }>;
    active?: boolean;
  }>
>([]);
const selectedSession = ref<SavedSessionDisplay | null>(null);

// Confirm dialog states
const showConfirmDialog = ref(false);
const confirmDialogTitle = ref('');
const confirmDialogMessage = ref('');
let pendingDeleteSession: SavedSessionDisplay | null = null;
let pendingDeleteSessionIds: string[] | null = null;
let pendingDeleteGroupId: string | null = null;

// Copy feedback state
const {
  message: copiedMessage,
  show: showCopyFeedback,
  clear: clearCopyFeedback,
} = useToastMessage(2000);

// XTerminal import dialog state
const showImportDialog = ref(false);

const openSSHForm = inject(OPEN_SSH_FORM_KEY);
const { t, locale } = useI18n();

// Create wrapper functions for event handlers that can be properly removed
const handleSessionSaved = async () => {
  if (isMounted.value) {
    await loadSessions();
  }
};

// Refresh the groups list from the backend, then reload sessions since they
// render group names.
const refreshGroups = async () => {
  if (!isMounted.value) return;
  try {
    const fetched = await invoke<Group[]>('list_groups');
    groups.value = fetched || [];
    await loadSessions();
  } catch (error) {
    logger.error('Failed to refresh groups', error);
  }
};

const handleGroupsUpdated = () => refreshGroups();

const toggleSessionSelection = (sessionId: string) => {
  const next = new Set(selectedSessionIds.value);
  if (next.has(sessionId)) {
    next.delete(sessionId);
  } else {
    next.add(sessionId);
  }
  selectedSessionIds.value = next;
};

// Session objects behind the current selection, used by batch operations.
const selectedSessions = computed(() =>
  sessions.value.filter(s => selectedSessionIds.value.has(s.id))
);

const allSelectedAreFavorites = computed(
  () =>
    selectedSessions.value.length > 0 &&
    selectedSessions.value.every(s => s.is_favorite)
);

const isSessionSelected = (sessionId: string) => {
  return selectedSessionIds.value.has(sessionId);
};

const toggleSelectAllGlobal = (event: Event) => {
  const checkbox = event.target as HTMLInputElement;
  const next = new Set(selectedSessionIds.value);
  if (checkbox.checked) {
    filteredSessions.value.forEach(s => next.add(s.id));
  } else {
    filteredSessions.value.forEach(s => next.delete(s.id));
  }
  selectedSessionIds.value = next;
};

// Reflect the filtered view's selection state in the header "select all"
// checkbox (checked when every visible row is selected, indeterminate when
// only some are).
const allSelected = computed(
  () =>
    filteredSessions.value.length > 0 &&
    filteredSessions.value.every(s => selectedSessionIds.value.has(s.id))
);

const someSelected = computed(
  () =>
    filteredSessions.value.some(s => selectedSessionIds.value.has(s.id)) &&
    !allSelected.value
);

// Fetch all sessions from the database in a single IPC round-trip.
const loadSessions = async () => {
  isLoading.value = true;
  try {
    const fetched = await invoke<SavedSessionDisplay[]>(
      'get_sessions_with_relations'
    );
    sessions.value = fetched || [];
    logger.info('Loaded sessions', { count: sessions.value.length });
  } catch (error) {
    logger.error('Failed to load sessions', error);
    sessions.value = [];
  } finally {
    isLoading.value = false;
  }
};

// Fetch groups from backend on mount
onMounted(async () => {
  isMounted.value = true;

  // Load sessions first
  await loadSessions();

  try {
    const fetchedGroups = await invoke<Group[]>('list_groups');
    groups.value = fetchedGroups || [];
  } catch (error) {
    logger.error('Failed to fetch groups', error);
  }

  // Listen for group updates from other components
  eventBus.on(APP_EVENTS.GROUPS_UPDATED, handleGroupsUpdated);
  // Reload sessions when a new session is saved
  eventBus.on(APP_EVENTS.SESSION_SAVED, handleSessionSaved);
});

onUnmounted(() => {
  isMounted.value = false;
  // Clean up event listeners
  eventBus.off(APP_EVENTS.GROUPS_UPDATED, handleGroupsUpdated);
  eventBus.off(APP_EVENTS.SESSION_SAVED, handleSessionSaved);
  clearCopyFeedback();
});

const handleNewConnection = () => {
  if (openSSHForm) openSSHForm();
};

const handleQuickConnect = async (session: SavedSessionDisplay) => {
  logger.info('Quick connect initiated', { session: session.server_name });
  eventBus.emit(APP_EVENTS.CONNECT_SESSION, session);
};

/**
 * Copy the session connection info to the clipboard, one labeled field per
 * line (name/address/port/user/password, labels localized via i18n).
 *
 * The password line is omitted when no plaintext can be obtained (e.g. key
 * auth only, or credential decryption failed).
 */
const handleCopySession = async (session: SavedSessionDisplay) => {
  try {
    const credentials = await invoke<
      [string, string | null, string | null]
    >('get_session_credentials', { sessionId: session.id }).catch(
      () => [session.id, null, null] as [string, string | null, string | null]
    );

    const lines = [
      `${t('home.copyName')}: ${session.server_name}`,
      `${t('home.copyAddr')}: ${session.addr}`,
      `${t('home.copyPort')}: ${session.port}`,
      `${t('home.copyUser')}: ${session.username}`,
    ];

    // Only copy the password when the plaintext is actually available.
    if (credentials[1]) {
      lines.push(`${t('home.copyPassword')}: ${credentials[1]}`);
    }

    await navigator.clipboard.writeText(lines.join('\n'));
    logger.info('Copied session info to clipboard', {
      session: session.server_name,
      hasPassword: !!credentials[1],
    });

    // Show transient feedback
    showCopyFeedback(t('home.copied'));
  } catch (error) {
    logger.error('Failed to copy session info', error);
  }
};

const handleDeleteGroup = (groupId: string) => {
  const group = groups.value.find(g => g.id === groupId);
  pendingDeleteGroupId = groupId;
  confirmDialogTitle.value = t('home.deleteGroup');
  confirmDialogMessage.value = group
    ? t('home.deleteGroupConfirm', { name: group.name })
    : '';
  showConfirmDialog.value = true;
};

const handleAddGroup = async () => {
  const name = newGroupName.value.trim();
  if (!name) return;

  try {
    await invoke('add_group', { name });
    newGroupName.value = '';
    showAddGroupInput.value = false;
    // Notify self (and any other views) so the new group shows immediately
    // instead of waiting for a restart.
    eventBus.emit(APP_EVENTS.GROUPS_UPDATED);
  } catch (error) {
    logger.error('Failed to add group', error);
  }
};

const handleGroupInputBlur = () => {
  // Discard if empty or when blurring
  showAddGroupInput.value = false;
  newGroupName.value = '';
};

const handleGroupInputKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') {
    handleAddGroup();
  } else if (e.key === 'Escape') {
    handleGroupInputBlur();
  }
};

const startEditGroup = (group: Group) => {
  editingGroupId.value = group.id;
  renamingGroupName.value = group.name;
};

const handleSaveGroupRename = async () => {
  if (!editingGroupId.value) return;
  const newName = renamingGroupName.value.trim();
  if (newName) {
    try {
      await invoke('edit_group', { id: editingGroupId.value, name: newName });
      const group = groups.value.find(g => g.id === editingGroupId.value);
      if (group) group.name = newName;
      // Also reload sessions since they might display group names
      await loadSessions();
    } catch (error) {
      logger.error('Failed to rename group', error);
    }
  }
  editingGroupId.value = null;
};

// Context menu handlers
const handleSessionContextMenu = (
  event: MouseEvent,
  session: SavedSessionDisplay
) => {
  // If the menu is already open for THIS session at THIS position, ignore
  if (
    contextMenuVisible.value &&
    selectedSession.value?.id === session.id &&
    contextMenuX.value === event.clientX &&
    contextMenuY.value === event.clientY
  ) {
    return;
  }

  // Set position and session first
  contextMenuX.value = event.clientX;
  contextMenuY.value = event.clientY;
  selectedSession.value = session;

  contextMenuItems.value = [
    {
      key: 'connect',
      label: t('common.connect') || 'Connect',
      icon: Terminal,
    },
    {
      key: 'file-manager',
      label: t('home.openFileManager') || 'Open File Manager',
      icon: FolderTree,
    },
    { key: 'divider', label: '', divider: true },
    {
      key: 'copy',
      label: t('home.copy') || 'Copy',
      icon: Copy,
    },
    { key: 'edit', label: t('common.edit') || 'Edit', icon: Pencil },
    { key: 'divider', label: '', divider: true },
    {
      key: 'join-group',
      label: t('home.groups') || 'Groups',
      icon: Folder,
      children:
        groups.value.length === 0
          ? [
              {
                key: '__empty__',
                label: t('connection.noGroupsAvailable'),
                disabled: true,
              },
            ]
          : groups.value.map(g => ({
              key: `group:${g.id}`,
              label: g.name,
              active: session.group_ids?.includes(g.id),
            })),
    },
    { key: 'divider', label: '', divider: true },
    {
      key: 'delete',
      label: t('common.delete') || 'Delete',
      danger: true,
      icon: Trash2,
    },
  ];

  // Open the menu
  contextMenuVisible.value = true;
};

const handleContextMenuSelect = async (key: string) => {
  // Skip divider
  if (key === 'divider') {
    return;
  }

  if (!selectedSession.value) {
    logger.warn('Context menu select with no session');
    return;
  }

  // Handle group linking
  if (key.startsWith('group:')) {
    const groupId = key.split(':')[1];
    const session = selectedSession.value;
    try {
      if (session.group_ids?.includes(groupId)) {
        await invoke('unlink_session_group', {
          sessionId: session.id,
          groupId,
        });
      } else {
        await invoke('link_session_group', {
          sessionId: session.id,
          groupId,
        });
      }
      await loadSessions();
    } catch (e) {
      logger.error('Failed to update session group', e);
    }
    return;
  }

  switch (key) {
    case 'connect':
      await handleQuickConnect(selectedSession.value);
      break;
    case 'file-manager':
      // The FM window dials its own SSH connection from the saved session's
      // credentials and releases it on close — no terminal tab needed.
      await openFileManagerWindow(selectedSession.value.id, {
        savedId: selectedSession.value.id,
      });
      break;
    case 'edit':
      handleEditSession(selectedSession.value);
      break;
    case 'copy':
      await handleCopySession(selectedSession.value);
      break;
    case 'delete':
      await handleDeleteSession(selectedSession.value);
      break;
    default:
      logger.warn('Unknown context menu action', { key });
  }
};

const handleEditSession = (session: SavedSessionDisplay) => {
  // Emit event to trigger edit session in App.vue
  eventBus.emit(APP_EVENTS.EDIT_SESSION, session);
};

// --- Batch operations (checkbox selection) ---
const batchGroupMenuOpen = ref(false);
const batchGroupX = ref(0);
const batchGroupY = ref(0);

// Position a batch submenu right below the clicked button.
const positionBatchMenu = (
  event: MouseEvent,
  width: number
): { x: number; y: number } => {
  const target = event.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  return {
    x: Math.min(rect.left, window.innerWidth - width),
    y: rect.bottom + 4,
  };
};

const toggleBatchGroupMenu = (event: MouseEvent) => {
  event.stopPropagation();
  if (!batchGroupMenuOpen.value) {
    const { x, y } = positionBatchMenu(event, 200);
    batchGroupX.value = x;
    batchGroupY.value = y;
  }
  batchGroupMenuOpen.value = !batchGroupMenuOpen.value;
};

// A group is "active" in the batch menu when EVERY selected session has it.
const batchGroupItems = computed(() =>
  groups.value.length === 0
    ? [
        {
          key: '__empty__',
          label: t('connection.noGroupsAvailable'),
          disabled: true,
        },
      ]
    : groups.value.map(g => ({
        key: `group:${g.id}`,
        label: g.name,
        active:
          selectedSessions.value.length > 0 &&
          selectedSessions.value.every(s => s.group_ids?.includes(g.id)),
      }))
);

const clearSelection = () => {
  selectedSessionIds.value = new Set();
};

const handleBatchGroupSelect = async (key: string) => {
  batchGroupMenuOpen.value = false;
  if (key === 'divider' || key === '__empty__') return;
  const groupId = key.split(':')[1];
  const ids = [...selectedSessionIds.value];
  const allHave = selectedSessions.value.every(s =>
    s.group_ids?.includes(groupId)
  );
  try {
    for (const id of ids) {
      await invoke(
        allHave ? 'unlink_session_group' : 'link_session_group',
        { sessionId: id, groupId }
      );
    }
    await loadSessions();
  } catch (e) {
    logger.error('Failed to batch update groups', e);
  }
};

const handleBatchFavorite = async () => {
  const newStatus = !allSelectedAreFavorites.value;
  try {
    for (const s of selectedSessions.value) {
      await sessionApi.toggleFavorite(s.id, newStatus);
      s.is_favorite = newStatus;
    }
    logger.info('Batch toggled favorite', {
      count: selectedSessions.value.length,
      isFavorite: newStatus,
    });
  } catch (e) {
    logger.error('Failed to batch toggle favorite', e);
  }
};

const handleBatchDelete = () => {
  pendingDeleteSessionIds = [...selectedSessionIds.value];
  confirmDialogTitle.value = t('common.delete');
  confirmDialogMessage.value = t('home.deleteSessionsConfirm', {
    count: pendingDeleteSessionIds.length,
  });
  showConfirmDialog.value = true;
};

const handleDeleteSession = (session: SavedSessionDisplay) => {
  pendingDeleteSession = session;
  confirmDialogTitle.value = t('home.deleteSession');
  confirmDialogMessage.value = t('home.deleteSessionConfirm', {
    name: session.server_name,
  });
  showConfirmDialog.value = true;
};

const onConfirmDelete = async () => {
  showConfirmDialog.value = false;

  if (pendingDeleteGroupId) {
    const id = pendingDeleteGroupId;
    pendingDeleteGroupId = null;
    try {
      await invoke('delete_group', { id });
      groups.value = groups.value.filter(g => g.id !== id);
      // If the deleted group was the active filter, fall back to the "all" view
      if (selectedGroupId.value === id) {
        selectedGroupId.value = null;
        activeView.value = 'all';
      }
      selectedSessionIds.value = new Set();
      // The backend cascades the group association away, so refresh the list
      await loadSessions();
    } catch (error) {
      logger.error('Failed to delete group', error);
    }
    return;
  }

  if (pendingDeleteSessionIds) {
    const ids = pendingDeleteSessionIds;
    pendingDeleteSessionIds = null;
    try {
      for (const id of ids) {
        await invoke('delete_session', { id });
      }
      selectedSessionIds.value = new Set();
      await loadSessions();
    } catch (error) {
      logger.error('Failed to batch delete sessions', error);
    }
    return;
  }

  if (!pendingDeleteSession) return;

  const session = pendingDeleteSession;
  pendingDeleteSession = null;

  try {
    await invoke('delete_session', { id: session.id });
    selectedSessionIds.value.delete(session.id);
    sessions.value = sessions.value.filter(s => s.id !== session.id);
    await loadSessions();
  } catch (error) {
    logger.error('Failed to delete session', error);
  }
};

const onCancelDelete = () => {
  showConfirmDialog.value = false;
  pendingDeleteSession = null;
  pendingDeleteSessionIds = null;
  pendingDeleteGroupId = null;
};
</script>

<style scoped>
.nexashell-home {
  height: 100%;
  display: flex;
  background: var(--color-bg-primary);
  overflow: hidden;
}

/* Sidebar Styles */
.home-sidebar {
  width: 220px;
  background: var(--color-bg-secondary);
  border-right: 1px solid var(--color-border-primary);
  display: flex;
  flex-direction: column;
  padding: 16px 8px;
  gap: 20px;
}

.section-title {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
  margin-bottom: 8px;
  padding: 0 10px;
  letter-spacing: 0.05em;
  font-weight: 600;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item-wrapper {
  display: flex;
  align-items: center;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px 6px 16px;
  border-radius: 0 20px 20px 0; /* Gmail style sidebar items */
  border: none;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 14px;
  transition: all 0.1s;
  flex: 1;
  text-align: left;
  margin-right: 8px;
}

.nav-item:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.nav-item.active {
  background: var(--color-interactive-selected);
  color: var(--color-primary);
  font-weight: 600;
}

.nav-item .count {
  margin-left: auto;
  font-size: 12px;
  font-weight: 400;
  color: var(--color-text-tertiary);
}

.nav-item.active .count {
  color: var(--color-primary);
  font-weight: 600;
}

.nav-item-wrapper:hover .item-actions {
  opacity: 1;
  visibility: visible;
}

.item-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  visibility: hidden;
  transition:
    opacity 0.2s,
    visibility 0.2s;
  padding-right: 4px;
}

.action-btn {
  background: none;
  border: none;
  color: var(--color-text-tertiary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.action-btn.small {
  padding: 4px;
}

.action-btn.tiny {
  padding: 2px;
}

.action-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.action-btn.delete:hover {
  color: var(--color-danger, #ef4444);
}

.rename-input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-md);
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  font-size: 14px;
  outline: none;
  margin: 2px 4px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.add-btn {
  background: none;
  border: none;
  color: var(--color-text-tertiary);
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  transition: all 0.2s;
}

.add-btn:hover {
  color: var(--color-primary);
}

.add-input-wrapper {
  padding: 0 12px 8px 12px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.add-actions {
  display: flex;
  gap: 2px;
}

.action-btn.confirm:hover {
  background: rgba(40, 200, 64, 0.15);
  color: var(--color-macos-maximize);
}

.action-btn.cancel:hover {
  background: rgba(255, 95, 87, 0.15);
  color: var(--color-macos-close);
}

.add-input {
  flex: 1;
  width: 0; /* Allow flex to shrink if needed */
  padding: 6px 8px;
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-md);
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  font-size: 13px;
  outline: none;
  transition: all 0.2s;
}

.add-input:focus {
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}

.add-input::placeholder {
  color: var(--color-text-tertiary);
}

/* Main Content Area Styles */
.home-main {
  flex: 1;
  overflow-y: auto;
  padding: 0; /* Remove padding to let table span full width */
  display: flex;
  flex-direction: column;
  background: var(--color-bg-primary);
}

.main-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 24px 24px 8px 24px;
}

.main-header h3 {
  font-size: 20px;
  margin: 0;
  color: var(--color-text-primary);
  font-weight: 500;
}

.main-header .subtitle {
  color: var(--color-text-secondary);
  margin: 2px 0 0 0;
  font-size: 13px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-action-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background: var(--color-bg-secondary);
  color: var(--color-text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}

.header-action-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  border-color: var(--color-border-secondary);
}

/* Batch action bar (appears when sessions are selected) */
.batch-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  margin: 0 24px 12px 24px;
  padding: 8px 12px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-md);
  flex-shrink: 0;
}

.batch-count {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  white-space: nowrap;
}

.batch-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.batch-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.batch-btn:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.batch-btn.danger:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

.action-area {
  display: flex;
  align-items: center;
  gap: 12px;
}

.btn-primary {
  background: var(--color-primary);
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: var(--radius-md);
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
  transition: all 0.2s;
  font-size: 13px;
}

.btn-primary:hover {
  background: var(--color-primary-hover);
}

/* Groups and Grids */
.session-manager {
  padding: 0;
}

.group-container {
  margin-bottom: 0;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 24px 8px 24px;
  background: var(--color-bg-secondary);
  border-bottom: 1px solid var(--color-border-primary);
  position: sticky;
  top: 0;
  z-index: 5;
}

.folder-icon {
  font-size: 16px;
  color: var(--color-text-tertiary);
}
.group-header .name {
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-text-tertiary);
}

.empty-state-container {
  padding: 40px;
  display: flex;
  justify-content: center;
}

.empty-card {
  border: 2px dashed var(--color-border-primary);
  background: transparent;
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--color-text-tertiary);
  cursor: pointer;
  width: 100%;
  max-width: 400px;
  min-height: 160px;
  padding: 24px;
  transition: all 0.2s;
}

.empty-card:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: var(--color-bg-secondary);
}

/* Table View Styles - Gmail inspired */
.session-table-wrapper {
  background: transparent;
}

.session-table {
  width: 100%;
  border-collapse: collapse;
}

.session-table thead th {
  text-align: left;
  padding: 12px;
  border-bottom: 1px solid var(--color-border-secondary);
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  background: var(--color-bg-primary);
}

.session-table thead th.col-drag,
.session-table thead th.col-checkbox,
.session-table thead th.col-favorite,
.session-table thead th.col-pin {
  padding: 12px 0;
}

.session-table td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border-subtle);
  color: var(--color-text-primary);
  font-size: 14px;
  transition: background-color 0.1s;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-row {
  cursor: pointer;
  background: var(--color-bg-primary);
  height: 48px;
  transition: background var(--transition-fast);
}

.session-row:hover {
  background: var(--color-interactive-hover);
}

.session-row.selected {
  background: var(--color-interactive-selected);
}

.col-drag {
  width: 32px;
  text-align: center;
  color: var(--color-text-tertiary);
  opacity: 0;
  padding: 0 !important;
}

.session-row:hover .col-drag {
  opacity: 1;
}

.drag-handle {
  /* Decorative only — there is no row-drag feature, so do not present a
   * draggable "grab" affordance. */
  cursor: default;
  display: inline-block;
}

.col-checkbox {
  width: 32px;
  text-align: center;
  padding: 0 4px !important;
}

.col-favorite {
  width: 32px;
  text-align: center;
  padding: 0 4px !important;
}

.col-pin {
  width: 32px;
  text-align: center;
  padding: 0 4px !important;
}

.col-name {
  width: 25%;
}

.col-host {
  width: 30%;
}

.col-last-connect {
  width: 140px;
  text-align: right;
}

.session-checkbox {
  appearance: none;
  width: 15px;
  height: 15px;
  border: 1px solid var(--color-border-primary);
  border-radius: 2px;
  background: transparent;
  cursor: pointer;
  position: relative;
  transition: all 0.2s;
  margin: 0;
  display: block;
}

.session-checkbox:hover {
  border-color: var(--color-text-tertiary);
}

.session-checkbox:checked {
  background: var(--color-primary);
  border-color: var(--color-primary);
}

.session-checkbox:checked::after {
  content: '';
  position: absolute;
  left: 4px;
  top: 1px;
  width: 4px;
  height: 8px;
  border: solid white;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
}

.favorite-btn-small {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  border-radius: 50%;
}

.favorite-btn-small:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
}

.favorite-btn-small.active {
  color: #facc15;
}

.pin-btn-small {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  border-radius: 50%;
}

.pin-btn-small:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
}

.pin-btn-small.active {
  color: var(--color-primary);
}

.name-text {
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}

.host-text {
  color: var(--color-text-tertiary);
  font-size: 13px;
}

.col-groups-list {
  width: 150px;
}

.table-groups {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.mini-group-tag {
  font-size: 11px;
  color: var(--color-text-secondary);
  background: var(--color-bg-tertiary);
  padding: 1px 6px;
  border-radius: 4px;
  border: 0.5px solid var(--color-border-primary);
}

.empty-text-dim {
  color: var(--color-text-placeholder);
  font-size: 12px;
}

.col-last-connect {
  width: 140px;
  text-align: right;
  padding-right: 16px !important;
}

.time-actions-wrapper {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  position: relative;
  height: 32px;
}

.timestamp {
  font-size: 12px;
  color: var(--color-text-tertiary);
  font-weight: 500;
  white-space: nowrap;
}

.icon-action-btn {
  background: transparent;
  border: none;
  color: var(--color-text-tertiary);
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.icon-action-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.home-toast {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  padding: 10px 18px;
  font-size: 13px;
  box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.25));
  pointer-events: none;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px);
}
</style>
