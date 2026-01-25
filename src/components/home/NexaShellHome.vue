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
                <Hash :size="12" :style="{ color: tag.color || 'inherit' }" />
                {{ tag.name }}
                <span v-if="tagCounts[tag.name] > 0" class="tag-count">
                  {{ tagCounts[tag.name] }}
                </span>
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

        <div v-else class="session-table-wrapper">
          <table class="session-table">
            <thead>
              <tr>
                <th class="col-drag"></th>
                <th class="col-checkbox">
                  <input
                    type="checkbox"
                    class="session-checkbox"
                    @change="toggleSelectAllGlobal($event)"
                  />
                </th>
                <th class="col-favorite"></th>
                <th class="col-name">{{ $t('home.serverName') }}</th>
                <th class="col-host">{{ $t('home.host') }}</th>
                <th class="col-groups-list">{{ $t('home.groups') }}</th>
                <th class="col-tags">{{ $t('home.tags') }}</th>
                <th class="col-last-connect">{{ $t('home.lastConnected') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="session in filteredSessions"
                :key="session.id"
                class="session-row"
                :class="{ selected: isSessionSelected(session.id) }"
                @click="handleConnect(session)"
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
                <td class="col-tags">
                  <div class="table-tags">
                    <span
                      v-for="tag in session.tags"
                      :key="tag"
                      class="mini-tag"
                      :style="getTagStyles(tag)"
                    >
                      {{ tag }}
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
                      <button
                        class="icon-action-btn more"
                        :title="$t('common.more') || 'More'"
                        @click.stop="handleSessionContextMenu($event, session)"
                      >
                        <MoreVertical :size="14" />
                      </button>
                    </div>
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
  Terminal,
  Pencil,
  Trash2,
  Check,
  GripVertical,
  MoreVertical,
} from 'lucide-vue-next';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import { OPEN_SSH_FORM_KEY } from '@/core/types';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';
import { formatRelativeTime } from '@/core/utils/time-utils';
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
const selectedSessionIds = ref<Set<string>>(new Set());

const favoriteCount = computed(
  () => sessions.value.filter(s => s.is_favorite).length
);

// Group and Tag counts for sidebar
const groupCounts = computed(() => {
  const counts: Record<string, number> = {};
  sessions.value.forEach(s => {
    if (s.groups && s.groups.length > 0) {
      s.groups.forEach(gName => {
        counts[gName] = (counts[gName] || 0) + 1;
      });
    }
  });
  return counts;
});

const tagCounts = computed(() => {
  const counts: Record<string, number> = {};
  sessions.value.forEach(s => {
    if (s.tags && s.tags.length > 0) {
      s.tags.forEach(tName => {
        counts[tName] = (counts[tName] || 0) + 1;
      });
    }
  });
  return counts;
});

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
    // Sort by last_connected_at descending and take only the first 10
    result = result
      .filter(s => s.last_connected_at)
      .sort((a, b) => {
        const dateA = a.last_connected_at
          ? new Date(a.last_connected_at.replace(' ', 'T') + 'Z').getTime()
          : 0;
        const dateB = b.last_connected_at
          ? new Date(b.last_connected_at.replace(' ', 'T') + 'Z').getTime()
          : 0;
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
  } else if (activeView.value === 'tag' && selectedTagId.value) {
    const tagName = tags.value.find(t => t.id === selectedTagId.value)?.name;
    if (tagName) {
      result = result.filter(s => s.tags?.includes(tagName));
    }
  }

  return result;
});

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
const { t, locale } = useI18n();

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

const toggleSessionSelection = (sessionId: string) => {
  if (selectedSessionIds.value.has(sessionId)) {
    selectedSessionIds.value.delete(sessionId);
  } else {
    selectedSessionIds.value.add(sessionId);
  }
};

const isSessionSelected = (sessionId: string) => {
  return selectedSessionIds.value.has(sessionId);
};

const toggleSelectAllGlobal = (event: Event) => {
  const checkbox = event.target as HTMLInputElement;
  if (checkbox.checked) {
    filteredSessions.value.forEach(s => selectedSessionIds.value.add(s.id));
  } else {
    filteredSessions.value.forEach(s => selectedSessionIds.value.delete(s.id));
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
    { key: 'edit', label: t('common.edit') || 'Edit', icon: Pencil },
    { key: 'divider', label: '', divider: true },
    {
      key: 'join-group',
      label: t('home.groups') || 'Groups',
      icon: Folder,
      children: groups.value.map(g => ({
        key: `group:${g.id}`,
        label: g.name,
        active: session.group_ids?.includes(g.id),
      })),
    },
    {
      key: 'join-tag',
      label: t('home.tags') || 'Tags',
      icon: Hash,
      children: tags.value.map(tag => ({
        key: `tag:${tag.id}`,
        label: tag.name,
        active: session.tag_ids?.includes(tag.id),
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

  // Handle group/tag linking
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
      console.error('Failed to update session group:', e);
    }
    return;
  }

  if (key.startsWith('tag:')) {
    const tagId = key.split(':')[1];
    const session = selectedSession.value;
    try {
      if (session.tag_ids?.includes(tagId)) {
        await invoke('unlink_session_tag', {
          sessionId: session.id,
          tagId,
        });
      } else {
        await invoke('link_session_tag', {
          sessionId: session.id,
          tagId,
        });
      }
      await loadSessions();
    } catch (e) {
      console.error('Failed to update session tag:', e);
    }
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
  console.log('NexaShellHome: handleEditSession', session.id);
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
  font-size: 13px;
  color: var(--color-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 12px;
  transition: all 0.2s;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
}

.tag-badge:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  border-color: var(--color-primary);
}

.tag-badge.active {
  background: var(--color-primary);
  color: white !important;
  border-color: var(--color-primary);
}

.tag-badge.active:hover {
  color: white;
  opacity: 0.9;
}

.tag-count {
  margin-left: 4px;
  font-size: 10px;
  opacity: 0.7;
  font-weight: 400;
}

.tag-badge.active .tag-count {
  opacity: 1;
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
  filter: brightness(1.1);
  box-shadow: 0 2px 8px rgba(var(--color-primary-rgb), 0.3);
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
.session-table thead th.col-favorite {
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
}

.session-row:hover {
  background: var(--color-bg-secondary);
}

.session-row:hover td:first-child {
  border-left: 3px solid var(--color-primary);
  padding-left: 9px; /* Compensate for the 3px border to keep alignment */
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
  cursor: grab;
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

.col-name {
  width: 25%;
}

.col-host {
  width: 30%;
}

.col-last-connect {
  width: 120px;
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

.col-tags {
  width: 180px;
  text-align: right;
}

.table-tags {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
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
  transition: opacity 0.1s ease;
}

.session-row:hover .timestamp {
  opacity: 0;
}

.action-buttons {
  position: absolute;
  right: 0;
  display: flex;
  gap: 2px;
  opacity: 0;
  transform: translateX(10px);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  pointer-events: none;
}

.session-row:hover .action-buttons {
  opacity: 1;
  transform: translateX(0);
  pointer-events: auto;
}

.icon-action-btn {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background-color 0.2s;
}

.icon-action-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.icon-action-btn.connect:hover {
  color: var(--color-primary);
  background: rgba(var(--color-primary-rgb), 0.1);
}

.icon-action-btn.delete:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

.mini-tag {
  font-size: 11px;
  font-weight: 500;
  background: var(--color-bg-tertiary);
  padding: 2px 8px;
  border-radius: 4px;
  color: var(--color-text-secondary);
  border: 1px solid transparent;
  white-space: nowrap;
}
</style>
