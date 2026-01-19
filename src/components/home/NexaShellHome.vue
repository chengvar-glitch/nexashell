<template>
  <div class="nexashell-home">
    <!-- Sidebar Navigation: Groups and Tags -->
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
            @keydown="handleGroupInputKeydown"
          />
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
                <Folder :size="16" />
                <span>{{ group.name }}</span>
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

      <div class="sidebar-section">
        <div class="section-header">
          <h4 class="section-title">
            {{ $t('home.tags') }}
          </h4>
          <button class="add-btn" @click="showAddTagInput = !showAddTagInput">
            <Plus :size="14" />
          </button>
        </div>
        <div v-if="showAddTagInput" class="add-input-wrapper">
          <input
            v-model="newTagName"
            type="text"
            class="add-input"
            placeholder="Tag name..."
            autofocus
            @keydown="handleTagInputKeydown"
          />
        </div>
        <div class="tag-cloud">
          <div v-for="tag in tags" :key="tag.id" class="tag-item-wrapper">
            <template v-if="editingTagId === tag.id">
              <div class="tag-edit-container">
                <input
                  v-model="renamingTagName"
                  type="text"
                  class="rename-tag-input"
                  autofocus
                  @keydown.enter="handleSaveTagRename"
                  @keydown.esc="editingTagId = null"
                />
                <div class="color-picker-mini">
                  <button
                    v-for="color in TAG_COLORS"
                    :key="color.value"
                    class="color-dot"
                    :class="{ active: renamingTagColor === color.value }"
                    :style="{
                      backgroundColor:
                        color.value || 'var(--color-bg-tertiary)',
                    }"
                    @click="renamingTagColor = color.value"
                  ></button>
                </div>
                <div class="edit-confirm-actions">
                  <button class="action-btn tiny" @click="handleSaveTagRename">
                    <Check :size="10" />
                  </button>
                </div>
              </div>
            </template>
            <template v-else>
              <span
                class="tag-badge"
                :class="{
                  active: activeView === 'tag' && selectedTagId === tag.id,
                }"
                :style="
                  activeView === 'tag' && selectedTagId === tag.id
                    ? {}
                    : { color: tag.color }
                "
                @click="setActiveTag(tag.id)"
              >
                <Hash :size="12" :style="{ color: tag.color || 'inherit' }" />{{
                  tag.name
                }}
              </span>
              <div class="item-actions">
                <button class="action-btn tiny" @click.stop="startEditTag(tag)">
                  <Pencil :size="10" />
                </button>
                <button
                  class="action-btn tiny delete"
                  @click.stop="handleDeleteTag(tag.id)"
                >
                  <Minus :size="10" />
                </button>
              </div>
            </template>
          </div>
        </div>
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
        <div class="action-area">
          <div class="view-toggle-wrapper">
            <button class="btn-secondary" @click="toggleViewMenu">
              <component
                :is="viewMode === 'grid' ? LayoutGrid : List"
                :size="18"
              />
              <ChevronDown :size="14" />
            </button>
            <DropdownMenu
              :visible="viewMenuVisible"
              :items="viewMenuItems"
              :x="viewMenuX"
              :y="viewMenuY"
              @update:visible="viewMenuVisible = $event"
              @select="handleViewModeSelect"
            />
          </div>
          <button class="btn-primary" @click="handleNewConnection">
            <Plus :size="18" /> {{ $t('home.newSession') }}
          </button>
        </div>
      </header>

      <!-- Session Grid/Table Area -->
      <section class="session-manager">
        <!-- Empty State (Globally if no sessions at all) -->
        <div v-if="filteredSessions.length === 0" class="empty-state-container">
          <button class="empty-card" @click="handleNewConnection">
            <Plus :size="32" class="plus" />
            <span>{{ $t('home.addFirst') }}</span>
          </button>
        </div>

        <div
          v-for="groupName in groupDisplayItems"
          v-else
          :key="groupName"
          class="group-container"
        >
          <div class="group-header">
            <FolderOpen :size="18" class="folder-icon" />
            <span class="name">{{ groupName }}</span>
          </div>

          <!-- Grid View -->
          <div v-if="viewMode === 'grid'" class="session-grid">
            <div
              v-for="session in getSessionsInGroup(groupName)"
              :key="session.id"
              class="session-card"
              @click="handleConnect(session)"
              @dblclick="handleQuickConnect(session)"
              @contextmenu.prevent="handleSessionContextMenu($event, session)"
            >
              <div class="card-top">
                <div class="avatar">
                  {{ (session.server_name || 'S')[0].toUpperCase() }}
                </div>
                <button
                  class="favorite-btn"
                  :class="{ active: session.is_favorite }"
                  @click.stop="toggleFavorite(session)"
                >
                  <Star
                    :size="18"
                    :fill="session.is_favorite ? 'currentColor' : 'none'"
                  />
                </button>
              </div>

              <div class="card-info">
                <div class="session-name">
                  {{ session.server_name }}
                </div>
                <div class="session-meta">
                  {{ session.username }}@{{ session.addr }}
                </div>
              </div>

              <div class="card-footer">
                <div class="session-tags">
                  <span
                    v-for="tag in session.tags"
                    :key="tag"
                    class="mini-tag"
                    :style="getTagStyles(tag)"
                  >
                    {{ tag }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- Table View -->
          <div v-else class="session-table-wrapper">
            <table class="session-table">
              <thead>
                <tr>
                  <th class="col-favorite"></th>
                  <th class="col-name">{{ $t('home.serverName') }}</th>
                  <th class="col-host">{{ $t('home.host') }}</th>
                  <th class="col-tags">{{ $t('home.tags') }}</th>
                  <th class="col-actions"></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="session in getSessionsInGroup(groupName)"
                  :key="session.id"
                  class="session-row"
                  @click="handleConnect(session)"
                  @dblclick="handleQuickConnect(session)"
                  @contextmenu.prevent="
                    handleSessionContextMenu($event, session)
                  "
                >
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
                  <td class="col-name">
                    <div class="name-with-icon">
                      <Server :size="14" class="row-icon" />
                      {{ session.server_name }}
                    </div>
                  </td>
                  <td class="col-host">
                    <span class="host-text"
                      >{{ session.username }}@{{ session.addr }}</span
                    >
                  </td>
                  <td class="col-tags">
                    <div class="table-tags">
                      <span
                        v-for="tag in session.tags"
                        :key="tag"
                        class="mini-tag compact"
                        :style="getTagStyles(tag)"
                      >
                        {{ tag }}
                      </span>
                    </div>
                  </td>
                  <td class="col-actions">
                    <div class="action-buttons">
                      <button
                        class="icon-action-btn connect"
                        :title="$t('common.connect')"
                        @click.stop="handleQuickConnect(session)"
                      >
                        <Terminal :size="14" />
                      </button>
                      <button
                        class="icon-action-btn edit"
                        :title="$t('common.edit') || 'Edit'"
                        @click.stop="handleEditSession(session)"
                      >
                        <Pencil :size="14" />
                      </button>
                      <button
                        class="icon-action-btn delete"
                        :title="$t('common.delete')"
                        @click.stop="handleDeleteSession(session)"
                      >
                        <Trash2 :size="14" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
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
  Hash,
  Minus,
  LayoutGrid,
  List,
  ChevronDown,
  Server,
  Terminal,
  Pencil,
  Trash2,
  Check,
} from 'lucide-vue-next';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import { OPEN_SSH_FORM_KEY } from '@/core/types';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';
import { sessionApi } from '@/features/session';
import type {
  SavedSession,
  SavedSessionDisplay,
} from '@/features/session/types';

interface Group {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

interface Tag {
  id: string;
  name: string;
  color?: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

// Predefined professional tag colors (Modern palette)
const TAG_COLORS = [
  { name: 'Default', value: '' },
  { name: 'Rose', value: '#f43f5e' },
  { name: 'Amber', value: '#f59e0b' },
  { name: 'Emerald', value: '#10b981' },
  { name: 'Cyan', value: '#06b6d4' },
  { name: 'Indigo', value: '#6366f1' },
  { name: 'Violet', value: '#8b5cf6' },
  { name: 'Fuchsia', value: '#d946ef' },
];

// Global states
const sessions = ref<SavedSessionDisplay[]>([]);
const groups = ref<Group[]>([]);
const tags = ref<Tag[]>([]);
const isMounted = ref(false);

// View and filter states
const activeView = ref<'all' | 'favorites' | 'recent' | 'group' | 'tag'>('all');
const selectedGroupId = ref<string | null>(null);
const selectedTagId = ref<string | null>(null);
const viewMode = ref<'grid' | 'table'>('table');

const favoriteCount = computed(
  () => sessions.value.filter(s => s.is_favorite).length
);

// View menu states
const viewMenuVisible = ref(false);
const viewMenuX = ref(0);
const viewMenuY = ref(0);
const viewMenuItems = computed(() => [
  {
    key: 'grid',
    label: t('home.gridView'),
    icon: LayoutGrid,
    active: viewMode.value === 'grid',
  },
  {
    key: 'table',
    label: t('home.tableView'),
    icon: List,
    active: viewMode.value === 'table',
  },
]);

const toggleViewMenu = (event: MouseEvent) => {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  viewMenuX.value = rect.left;
  viewMenuY.value = rect.bottom + 8;
  viewMenuVisible.value = !viewMenuVisible.value;
};

const handleViewModeSelect = (key: string) => {
  if (key === 'grid' || key === 'table') {
    viewMode.value = key;
  }
};

// Input states for adding new groups/tags
const showAddGroupInput = ref(false);
const newGroupName = ref('');
const showAddTagInput = ref(false);
const newTagName = ref('');

// Renaming states
const editingGroupId = ref<string | null>(null);
const renamingGroupName = ref('');
const editingTagId = ref<string | null>(null);
const renamingTagName = ref('');
const renamingTagColor = ref('');

// Computed values for UI
const viewTitle = computed(() => {
  switch (activeView.value) {
    case 'favorites':
      return t('home.favorites');
    case 'recent':
      return t('home.recent');
    case 'group':
      return (
        groups.value.find(g => g.id === selectedGroupId.value)?.name || 'Group'
      );
    case 'tag':
      return tags.value.find(t => t.id === selectedTagId.value)?.name || 'Tag';
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
    // Sort by updated_at descending and take only the first 5
    result = result
      .filter(s => s.updated_at)
      .sort((a, b) => {
        const dateA = a.updated_at ? new Date(a.updated_at).getTime() : 0;
        const dateB = b.updated_at ? new Date(b.updated_at).getTime() : 0;
        return dateB - dateA;
      })
      .slice(0, 5);
  } else if (activeView.value === 'group' && selectedGroupId.value) {
    const groupName = groups.value.find(
      g => g.id === selectedGroupId.value
    )?.name;
    if (groupName) {
      result = result.filter(s => s.groups?.includes(groupName));
    }
  } else if (activeView.value === 'tag' && selectedTagId.value) {
    const tagName = tags.value.find(t => t.id === selectedTagId.value)?.name;
    if (tagName) {
      result = result.filter(s => s.tags?.includes(tagName));
    }
  }

  return result;
});

// Calculate which group headers to display
const groupDisplayItems = computed(() => {
  if (activeView.value === 'group') {
    const groupName = groups.value.find(
      g => g.id === selectedGroupId.value
    )?.name;
    return groupName ? [groupName] : [];
  }

  const groupSet = new Set<string>();
  filteredSessions.value.forEach(s => {
    if (s.groups && s.groups.length > 0) {
      s.groups.forEach(g => groupSet.add(g));
    } else {
      groupSet.add(t('home.defaultGroup'));
    }
  });

  return Array.from(groupSet).sort((a, b) => {
    if (a === t('home.defaultGroup')) return -1;
    if (b === t('home.defaultGroup')) return 1;
    return a.localeCompare(b);
  });
});

const getSessionsInGroup = (groupName: string) => {
  if (groupName === t('home.defaultGroup')) {
    return filteredSessions.value.filter(
      s => !s.groups || s.groups.length === 0
    );
  }
  return filteredSessions.value.filter(s => s.groups?.includes(groupName));
};

// Selection handlers
const getTagStyles = (tagName: string) => {
  const tag = tags.value.find(t => t.name === tagName);
  if (!tag?.color) return {};

  // For colors, we create a soft background and a matching border
  return {
    backgroundColor: `${tag.color}15`, // 15% opacity hex
    borderColor: `${tag.color}40`, // 25% opacity hex for border
    color: tag.color,
  };
};

const setActiveView = (view: 'all' | 'favorites' | 'recent') => {
  activeView.value = view;
  selectedGroupId.value = null;
  selectedTagId.value = null;
};

const setActiveGroup = (groupId: string) => {
  activeView.value = 'group';
  selectedGroupId.value = groupId;
  selectedTagId.value = null;
};

const setActiveTag = (tagId: string) => {
  activeView.value = 'tag';
  selectedTagId.value = tagId;
  selectedGroupId.value = null;
};

const toggleFavorite = async (session: SavedSessionDisplay) => {
  try {
    const newStatus = !session.is_favorite;
    await sessionApi.toggleFavorite(session.id, newStatus);
    session.is_favorite = newStatus;
    console.log(
      'Toggled favorite for session:',
      session.server_name,
      newStatus
    );
  } catch (error) {
    console.error('Failed to toggle favorite:', error);
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
  }>
>([]);
const selectedSession = ref<SavedSessionDisplay | null>(null);

// Confirm dialog states
const showConfirmDialog = ref(false);
const confirmDialogTitle = ref('');
const confirmDialogMessage = ref('');
let pendingDeleteSession: SavedSessionDisplay | null = null;

const openSSHForm = inject(OPEN_SSH_FORM_KEY);
const { t } = useI18n();

// Create wrapper functions for event handlers that can be properly removed
const handleSessionSaved = async () => {
  console.log('SESSION_SAVED event received, isMounted:', isMounted.value);
  if (isMounted.value) {
    console.log('Component is mounted, reloading sessions...');
    await loadSessions();
  } else {
    console.log('Component not yet mounted, will load on mount');
  }
};

const handleGroupsUpdated = async () => {
  console.log('GROUPS_UPDATED event received');
  if (!isMounted.value) return;
  try {
    const fetchedGroups = await invoke<Group[]>('list_groups');
    groups.value = fetchedGroups || [];
    console.log('Refreshed groups:', fetchedGroups);
    // Reload sessions when groups change
    await loadSessions();
  } catch (error) {
    console.error('Failed to refresh groups:', error);
  }
};

const handleTagsUpdated = async () => {
  console.log('TAGS_UPDATED event received');
  if (!isMounted.value) return;
  try {
    const fetchedTags = await invoke<Tag[]>('list_tags');
    tags.value = fetchedTags || [];
    console.log('Refreshed tags:', fetchedTags);
    // Reload sessions when tags change
    await loadSessions();
  } catch (error) {
    console.error('Failed to refresh tags:', error);
  }
};

// Fetch all sessions from the database
const loadSessions = async () => {
  try {
    const dbSessions = await invoke<SavedSession[]>('list_sessions');

    if (!dbSessions || dbSessions.length === 0) {
      sessions.value = [];
      return;
    }

    // Transform database sessions to UI format and fetch associated groups/tags
    const transformedSessions: SavedSessionDisplay[] = await Promise.all(
      dbSessions.map(async dbSession => {
        try {
          // Fetch groups for this session
          const sessionGroups = await invoke<
            Array<{ id: string; name: string }>
          >('list_groups_for_session', {
            sessionId: dbSession.id,
          });

          // Fetch tags for this session
          const sessionTags = await invoke<Array<{ id: string; name: string }>>(
            'list_tags_for_session',
            {
              sessionId: dbSession.id,
            }
          );

          return {
            ...dbSession,
            groups: sessionGroups?.map(g => g.name) || [],
            group_ids: sessionGroups?.map(g => g.id) || [],
            tags: sessionTags?.map(t => t.name) || [],
            tag_ids: sessionTags?.map(t => t.id) || [],
          } as SavedSessionDisplay;
        } catch (error) {
          console.error(
            `Failed to fetch groups/tags for session ${dbSession.id}:`,
            error
          );
          return {
            ...dbSession,
            groups: [],
            tags: [],
          } as SavedSessionDisplay;
        }
      })
    );

    sessions.value = transformedSessions;
    console.log('Loaded sessions from database:', transformedSessions);
  } catch (error) {
    console.error('Failed to load sessions:', error);
    sessions.value = [];
  }
};

// Fetch groups and tags from backend on mount
onMounted(async () => {
  console.log('NexaShellHome mounted');
  isMounted.value = true;

  // Load sessions first
  await loadSessions();

  try {
    const fetchedGroups = await invoke<Group[]>('list_groups');
    groups.value = fetchedGroups || [];
    console.log('Loaded groups:', fetchedGroups);
  } catch (error) {
    console.error('Failed to fetch groups:', error);
  }

  try {
    const fetchedTags = await invoke<Tag[]>('list_tags');
    tags.value = fetchedTags || [];
    console.log('Loaded tags:', fetchedTags);
  } catch (error) {
    console.error('Failed to fetch tags:', error);
  }

  // Listen for group and tag updates from other components
  console.log('Registering event listeners...');
  eventBus.on(APP_EVENTS.GROUPS_UPDATED, handleGroupsUpdated);
  eventBus.on(APP_EVENTS.TAGS_UPDATED, handleTagsUpdated);
  // Reload sessions when a new session is saved
  eventBus.on(APP_EVENTS.SESSION_SAVED, handleSessionSaved);
  console.log('Event listeners registered');
});

onUnmounted(() => {
  console.log('NexaShellHome unmounting');
  isMounted.value = false;
  // Clean up event listeners
  eventBus.off(APP_EVENTS.GROUPS_UPDATED, handleGroupsUpdated);
  eventBus.off(APP_EVENTS.TAGS_UPDATED, handleTagsUpdated);
  eventBus.off(APP_EVENTS.SESSION_SAVED, handleSessionSaved);
});

const handleNewConnection = () => {
  if (openSSHForm) openSSHForm();
};

const handleConnect = (session: SavedSessionDisplay) => {
  // Single click to select, double click to connect
  console.log('Session selected:', session.server_name);
};

const handleQuickConnect = async (session: SavedSessionDisplay) => {
  console.log('Quick connect initiated for session:', session.server_name);
  eventBus.emit(APP_EVENTS.CONNECT_SESSION, session);
};

const handleDeleteGroup = async (groupId: string) => {
  try {
    await invoke('delete_group', { id: groupId });
    groups.value = groups.value.filter(g => g.id !== groupId);
    // The backend cascades deletion to session_groups, clearing SSH sessions' group association
  } catch (error) {
    console.error('Failed to delete group:', error);
  }
};

const handleDeleteTag = async (tagId: string) => {
  try {
    await invoke('delete_tag', { id: tagId });
    tags.value = tags.value.filter(t => t.id !== tagId);
    // The backend cascades deletion to session_tags, clearing SSH sessions' tag association
  } catch (error) {
    console.error('Failed to delete tag:', error);
  }
};

const handleAddGroup = async () => {
  const name = newGroupName.value.trim();
  if (!name) return;

  try {
    await invoke('add_group', { name });
    newGroupName.value = '';
    showAddGroupInput.value = false;
    // Event listener will handle refresh
  } catch (error) {
    console.error('Failed to add group:', error);
  }
};

const handleAddTag = async () => {
  const name = newTagName.value.trim();
  if (!name) return;

  try {
    await invoke('add_tag', { name });
    newTagName.value = '';
    showAddTagInput.value = false;
    // Event listener will handle refresh
  } catch (error) {
    console.error('Failed to add tag:', error);
  }
};

const handleGroupInputKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') {
    handleAddGroup();
  } else if (e.key === 'Escape') {
    showAddGroupInput.value = false;
    newGroupName.value = '';
  }
};

const handleTagInputKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') {
    handleAddTag();
  } else if (e.key === 'Escape') {
    showAddTagInput.value = false;
    newTagName.value = '';
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
      console.error('Failed to rename group:', error);
    }
  }
  editingGroupId.value = null;
};

const startEditTag = (tag: Tag) => {
  editingTagId.value = tag.id;
  renamingTagName.value = tag.name;
  renamingTagColor.value = tag.color || '';
};

const handleSaveTagRename = async () => {
  if (!editingTagId.value) return;
  const newName = renamingTagName.value.trim();
  const newColor = renamingTagColor.value;
  if (newName) {
    try {
      await invoke('edit_tag', {
        id: editingTagId.value,
        name: newName,
        color: newColor,
      });
      const tag = tags.value.find(t => t.id === editingTagId.value);
      if (tag) {
        tag.name = newName;
        tag.color = newColor;
      }
      // Also reload sessions since they might display tag names/colors
      await loadSessions();
    } catch (error) {
      console.error('Failed to update tag:', error);
    }
  }
  editingTagId.value = null;
};

// Context menu handlers
const handleSessionContextMenu = (
  event: MouseEvent,
  session: SavedSessionDisplay
) => {
  console.log(
    'Context menu opened for session:',
    session.id,
    session.server_name
  );
  contextMenuX.value = event.clientX;
  contextMenuY.value = event.clientY;
  selectedSession.value = session;
  contextMenuItems.value = [
    { key: 'edit', label: 'Edit' },
    { key: 'divider', label: '', divider: true },
    { key: 'delete', label: 'Delete', danger: true },
  ];
  contextMenuVisible.value = true;
  console.log('Context menu visible, items:', contextMenuItems.value);
};

const handleContextMenuSelect = async (key: string) => {
  console.log('Context menu item selected:', key);

  // Skip divider
  if (key === 'divider') {
    console.log('Skipping divider');
    return;
  }

  console.log('Selected session:', selectedSession.value);

  if (!selectedSession.value) {
    console.warn('No session selected');
    return;
  }

  switch (key) {
    case 'edit':
      console.log('Handling edit for:', selectedSession.value.id);
      handleEditSession(selectedSession.value);
      break;
    case 'delete':
      console.log('Handling delete for:', selectedSession.value.id);
      await handleDeleteSession(selectedSession.value);
      break;
    default:
      console.log('Unknown menu action:', key);
  }
};

const handleEditSession = (session: SavedSessionDisplay) => {
  console.log('Edit session:', session.id);
  // Emit event to trigger edit session in App.vue
  eventBus.emit(APP_EVENTS.EDIT_SESSION, session);
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
  if (!pendingDeleteSession) return;

  showConfirmDialog.value = false;
  const session = pendingDeleteSession;
  pendingDeleteSession = null;

  try {
    console.log('Invoking delete_session for session ID:', session.id);
    const result = await invoke('delete_session', { id: session.id });
    console.log('Delete result:', result);

    sessions.value = sessions.value.filter(s => s.id !== session.id);
    await loadSessions();
  } catch (error) {
    console.error('Failed to delete session:', error);
  }
};

const onCancelDelete = () => {
  showConfirmDialog.value = false;
  pendingDeleteSession = null;
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
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  border: none;
  background: transparent;
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
  flex: 1;
  text-align: left;
}

.nav-item:hover {
  background: var(--color-bg-tertiary);
}

.nav-item.active {
  background: var(--color-bg-tertiary);
  color: var(--color-primary);
  font-weight: 500;
}

.nav-item .count {
  margin-left: auto;
  font-size: 11px;
  background: var(--color-bg-tertiary);
  padding: 2px 6px;
  border-radius: 10px;
}

.nav-item-wrapper:hover .item-actions,
.tag-item-wrapper:hover .item-actions {
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

.rename-tag-input {
  width: 80px;
  padding: 2px 6px;
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-sm);
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  font-size: 12px;
  outline: none;
}

.tag-edit-container {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  z-index: 10;
}

.color-picker-mini {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 6px;
  background: var(--color-bg-secondary);
  border-radius: 6px;
  border: 1px solid var(--color-border-primary);
  margin-bottom: 4px;
}

.color-dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.1);
}

.color-dot:hover {
  transform: scale(1.2);
}

.color-dot.active {
  border-color: #fff;
  box-shadow:
    0 0 0 1.5px var(--color-primary),
    0 2px 4px rgba(0, 0, 0, 0.1);
  transform: scale(1.1);
}

.edit-confirm-actions {
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid var(--color-border-secondary);
  padding-top: 4px;
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
}

.add-input {
  width: 100%;
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
  box-shadow: 0 0 0 2px rgba(var(--color-primary-rgb), 0.1);
}

.add-input::placeholder {
  color: var(--color-text-tertiary);
}

.tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0 12px;
}

.tag-item-wrapper {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tag-badge {
  font-size: 12px;
  color: var(--color-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}

.tag-badge:hover {
  color: var(--color-primary);
}

.tag-badge.active {
  background: var(--color-primary);
  color: white;
}

.tag-badge.active:hover {
  color: white;
  opacity: 0.9;
}

/* Main Content Area Styles */
.home-main {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.main-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.main-header h3 {
  font-size: 20px;
  margin: 0;
  color: var(--color-text-primary);
}

.main-header .subtitle {
  color: var(--color-text-secondary);
  margin: 2px 0 0 0;
  font-size: 13px;
}

.action-area {
  display: flex;
  align-items: center;
  gap: 12px;
}

.view-toggle-wrapper {
  position: relative;
}

.btn-secondary {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
  padding: 10px 14px;
  border-radius: var(--radius-md);
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
  transition: all 0.2s;
}

.btn-secondary:hover {
  background: var(--color-bg-tertiary);
  border-color: var(--color-primary);
}

.btn-primary {
  background: var(--color-primary);
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: var(--radius-md);
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
}

.btn-primary:hover {
  filter: brightness(1.1);
}

/* Groups and Grids */
.group-container {
  margin-bottom: 24px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border-primary);
}

.folder-icon {
  font-size: 18px;
  color: var(--color-primary);
}
.group-header .name {
  font-weight: 600;
  font-size: 14px;
  color: var(--color-text-primary);
}

.session-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}

.empty-state-container {
  grid-column: 1 / -1;
  width: 100%;
}

.session-card {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  padding: 12px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  flex-direction: column;
  gap: 8px;
  user-select: none;
}

.session-card:hover {
  border-color: var(--color-primary);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.avatar {
  width: 32px;
  height: 32px;
  background: var(--color-bg-tertiary);
  color: var(--color-primary);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  font-size: 14px;
}

.favorite-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  transition: all 0.2s;
  padding: 4px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
}

.favorite-btn:hover {
  background: var(--color-bg-tertiary);
  color: #facc15; /* Yellow/Gold */
}

.favorite-btn.active {
  color: #facc15;
}

.icon-btn {
  background: none;
  border: none;
  cursor: pointer;
  opacity: 0.3;
  transition: 0.2s;
  font-size: 16px;
}

.icon-btn:hover {
  opacity: 1;
}

.session-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.session-meta {
  font-size: 12px;
  color: var(--color-text-tertiary);
  margin-top: 4px;
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
    'Courier New', monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: auto;
  border-top: 1px solid var(--color-bg-tertiary);
  padding-top: 8px;
}

.session-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  max-height: 24px;
  overflow: hidden;
}

.mini-tag {
  font-size: 11px;
  font-weight: 500;
  background: var(--color-bg-tertiary);
  padding: 3px 10px;
  border-radius: 6px;
  color: var(--color-text-secondary);
  border: 1px solid var(--color-border-primary);
  white-space: nowrap;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.mini-tag.compact {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 4px;
}

.mini-tag:hover {
  transform: translateY(-1px);
  box-shadow:
    0 4px 6px -1px rgba(0, 0, 0, 0.1),
    0 2px 4px -1px rgba(0, 0, 0, 0.06);
}

.connect-hint {
  font-size: 11px;
  color: var(--color-primary);
  font-weight: 500;
  opacity: 0;
  transition: 0.2s;
  display: flex;
  align-items: center;
  gap: 2px;
}

.session-card:hover .connect-hint {
  opacity: 1;
}

.empty-card {
  border: 2px dashed var(--color-border-primary);
  background: transparent;
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-text-tertiary);
  cursor: pointer;
  min-height: 110px;
  padding: 12px;
  transition: 0.2s;
}

.empty-card span {
  font-size: 13px;
  text-align: center;
}

.empty-card .plus {
  font-size: 24px;
}

/* Modal styles for quick connect progress */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(2px);
  background: rgba(0, 0, 0, 0.3);
}

.modal-content {
  position: relative;
  border: none;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.15);
  border-radius: var(--radius-lg);
  overflow: hidden;
  animation: modal-appear 0.2s ease-out forwards;
}

@keyframes modal-appear {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

/* Table View Styles */
.session-table-wrapper {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.session-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
}

.session-table th {
  text-align: left;
  padding: 8px 12px;
  background: var(--color-bg-tertiary);
  color: var(--color-text-tertiary);
  font-weight: 600;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid var(--color-border-primary);
}

.session-table td {
  padding: 6px 12px;
  border-bottom: 1px solid var(--color-border-primary);
  color: var(--color-text-primary);
  font-size: 13px;
}

.session-row {
  cursor: pointer;
  transition: background 0.15s;
}

.session-row:hover {
  background: var(--color-bg-tertiary);
}

.session-row:last-child td {
  border-bottom: none;
}

.col-favorite {
  width: 32px;
  text-align: center;
}

.favorite-btn-small {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  padding: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s;
}

.favorite-btn-small:hover,
.favorite-btn-small.active {
  color: #facc15;
}

.col-name {
  font-weight: 500;
  width: 25%;
}

.name-with-icon {
  display: flex;
  align-items: center;
  gap: 6px;
}

.row-icon {
  color: var(--color-primary);
}

.col-host {
  width: 35%;
  color: var(--color-text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.table-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.col-actions {
  text-align: right;
  width: 120px;
}

.action-buttons {
  display: flex;
  justify-content: flex-end;
  gap: 4px;
}

.icon-action-btn {
  background: transparent;
  border: none;
  color: var(--color-text-tertiary);
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.icon-action-btn:hover {
  background: var(--color-bg-elevated);
  color: var(--color-text-primary);
}

.icon-action-btn.connect {
  color: var(--color-primary);
  background: var(--color-interactive-selected);
}

.icon-action-btn.connect:hover {
  background: var(--color-primary);
  color: white;
}

.icon-action-btn.delete:hover {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

.row-action-btn {
  background: transparent;
  border: 1px solid var(--color-primary);
  color: var(--color-primary);
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.row-action-btn:hover {
  background: var(--color-primary);
  color: white;
}
</style>
