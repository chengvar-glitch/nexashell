<template>
  <div class="nexashell-home">
    <!-- Sidebar Navigation: Groups and Tags -->
    <aside class="home-sidebar">
      <div class="sidebar-section">
        <h4 class="section-title">
          {{ $t('home.views') }}
        </h4>
        <nav class="sidebar-nav">
          <button class="nav-item active">
            <Home :size="16" />
            <span>{{ $t('home.allSessions') }}</span>
            <span class="count">{{ sessions.length }}</span>
          </button>
          <button class="nav-item">
            <Star :size="16" />
            <span>{{ $t('home.favorites') }}</span>
          </button>
          <button class="nav-item">
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
          <button class="add-btn" @click="showAddGroupInput = !showAddGroupInput">
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
            <button class="nav-item">
              <Folder :size="16" />
              <span>{{ group.name }}</span>
            </button>
            <button class="delete-btn" @click.stop="handleDeleteGroup(group.id)">
              <Minus :size="14" />
            </button>
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
            <span class="tag-badge">
              <Hash :size="12" />{{ tag.name }}
            </span>
            <button class="tag-delete-btn" @click.stop="handleDeleteTag(tag.id)">
              <Minus :size="12" />
            </button>
          </div>
        </div>
      </div>
    </aside>

    <!-- Main Content: Session Management -->
    <main class="home-main">
      <header class="main-header">
        <div class="title-area">
          <h3>{{ $t('home.allSessions') }}</h3>
          <p class="subtitle">
            {{ $t('home.subtitle') }}
          </p>
        </div>
        <div class="action-area">
          <button class="btn-primary" @click="handleNewConnection">
            <Plus :size="18" /> {{ $t('home.newSession') }}
          </button>
        </div>
      </header>

      <!-- Session Grid Area -->
      <section class="session-manager">
        <div
          v-for="groupName in [$t('home.defaultGroup')]"
          :key="groupName"
          class="group-container"
        >
          <div class="group-header">
            <FolderOpen :size="18" class="folder-icon" />
            <span class="name">{{ groupName }}</span>
          </div>

          <div class="session-grid">
            <div
              v-for="session in sessions"
              :key="session.id"
              class="session-card"
              @click="handleConnect(session)"
            >
              <div class="card-top">
                <div class="avatar">
                  {{ session.name[0].toUpperCase() }}
                </div>
              </div>

              <div class="card-info">
                <div class="session-name">
                  {{ session.name }}
                </div>
                <div class="session-meta">
                  {{ session.username }}@{{ session.host }}
                </div>
              </div>

              <div class="card-footer">
                <div class="session-tags">
                  <span
                    v-for="tag in session.tags"
                    :key="tag"
                    class="mini-tag"
                    >{{ tag }}</span
                  >
                </div>
                <div class="connect-hint">
                  {{ $t('home.connect') }} <ChevronRight :size="14" />
                </div>
              </div>
            </div>

            <!-- Empty State -->
            <button
              v-if="sessions.length === 0"
              class="empty-card"
              @click="handleNewConnection"
            >
              <Plus :size="32" class="plus" />
              <span>{{ $t('home.addFirst') }}</span>
            </button>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, inject, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  Home,
  Star,
  History,
  Folder,
  FolderOpen,
  Plus,
  ChevronRight,
  Hash,
  Minus,
} from 'lucide-vue-next';
import { OPEN_SSH_FORM_KEY } from '@/core/types';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';

interface SSHSession {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group?: string;
  tags?: string[];
}

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
  sort: number;
  created_at: string;
  updated_at: string;
}

// Mock sessions data (keep for now)
const sessions = ref<SSHSession[]>([
  {
    id: '1',
    name: 'Production Web 01',
    host: '192.168.1.10',
    port: 22,
    username: 'root',
    group: 'Production',
    tags: ['Web', 'Nginx'],
  },
  {
    id: '2',
    name: 'Database Master',
    host: 'db.example.com',
    port: 22,
    username: 'admin',
    group: 'Core',
    tags: ['DB', 'MySQL'],
  },
]);

// Real data from backend
const groups = ref<Group[]>([]);
const tags = ref<Tag[]>([]);

// Input states for adding new groups/tags
const showAddGroupInput = ref(false);
const newGroupName = ref('');
const showAddTagInput = ref(false);
const newTagName = ref('');

const openSSHForm = inject<() => void>(OPEN_SSH_FORM_KEY);

// Fetch groups and tags from backend on mount
onMounted(async () => {
  try {
    const fetchedGroups = await invoke<Group[]>('list_groups');
    groups.value = fetchedGroups || [];
  } catch (error) {
    console.error('Failed to fetch groups:', error);
  }

  try {
    const fetchedTags = await invoke<Tag[]>('list_tags');
    tags.value = fetchedTags || [];
  } catch (error) {
    console.error('Failed to fetch tags:', error);
  }

  // Listen for group and tag updates from other components
  eventBus.on(APP_EVENTS.GROUPS_UPDATED, refreshGroups);
  eventBus.on(APP_EVENTS.TAGS_UPDATED, refreshTags);
});

onUnmounted(() => {
  // Clean up event listeners
  eventBus.off(APP_EVENTS.GROUPS_UPDATED, refreshGroups);
  eventBus.off(APP_EVENTS.TAGS_UPDATED, refreshTags);
});

const refreshGroups = async () => {
  try {
    const fetchedGroups = await invoke<Group[]>('list_groups');
    groups.value = fetchedGroups || [];
  } catch (error) {
    console.error('Failed to refresh groups:', error);
  }
};

const refreshTags = async () => {
  try {
    const fetchedTags = await invoke<Tag[]>('list_tags');
    tags.value = fetchedTags || [];
  } catch (error) {
    console.error('Failed to refresh tags:', error);
  }
};

const handleNewConnection = () => {
  if (openSSHForm) openSSHForm();
};

const handleConnect = (_session: SSHSession) => {
  // TODO: emit connect event
};

const handleDeleteGroup = async (groupId: string) => {
  try {
    await invoke('delete_group', { id: groupId });
    groups.value = groups.value.filter((g) => g.id !== groupId);
    // The backend cascades deletion to session_groups, clearing SSH sessions' group association
  } catch (error) {
    console.error('Failed to delete group:', error);
  }
};

const handleDeleteTag = async (tagId: string) => {
  try {
    await invoke('delete_tag', { id: tagId });
    tags.value = tags.value.filter((t) => t.id !== tagId);
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
</script>

<style scoped>
.nexashell-home {
  height: 100%;
  display: flex;
  background: var(--color-bg-primary);
  overflow: hidden;
}

/* 侧边栏样式 */
.home-sidebar {
  width: 240px;
  background: var(--color-bg-secondary);
  border-right: 1px solid var(--color-border-primary);
  display: flex;
  flex-direction: column;
  padding: 24px 12px;
  gap: 32px;
}

.section-title {
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
  margin-bottom: 12px;
  padding: 0 12px;
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

.delete-btn {
  background: none;
  border: none;
  color: var(--color-text-tertiary);
  cursor: pointer;
  padding: 6px;
  border-radius: var(--radius-md);
  opacity: 0;
  transition: all 0.2s;
}

.nav-item-wrapper:hover .delete-btn {
  opacity: 1;
  color: var(--color-text-secondary);
}

.delete-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-danger, #ef4444);
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

.tag-delete-btn {
  background: none;
  border: none;
  color: var(--color-text-tertiary);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  opacity: 0;
  transition: all 0.2s;
  line-height: 1;
}

.tag-item-wrapper:hover .tag-delete-btn {
  opacity: 1;
  color: var(--color-text-secondary);
}

.tag-delete-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-danger, #ef4444);
}

/* 主内容区样式 */
.home-main {
  flex: 1;
  overflow-y: auto;
  padding: 40px;
  display: flex;
  flex-direction: column;
  gap: 40px;
}

.main-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.main-header h3 {
  font-size: 24px;
  margin: 0;
  color: var(--color-text-primary);
}

.main-header .subtitle {
  color: var(--color-text-secondary);
  margin: 4px 0 0 0;
  font-size: 14px;
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
}

/* 分组与网格 */
.group-container {
  margin-bottom: 32px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--color-border-primary);
}

.folder-icon {
  font-size: 18px;
}
.group-header .name {
  font-weight: 600;
  font-size: 14px;
  color: var(--color-text-primary);
}

.session-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 20px;
}

.session-card {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
  padding: 24px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.session-card:hover {
  border-color: var(--color-primary);
  transform: translateY(-4px);
  box-shadow: 0 12px 24px rgba(0, 0, 0, 0.1);
}

.card-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.avatar {
  width: 44px;
  height: 44px;
  background: var(--color-bg-tertiary);
  color: var(--color-primary);
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  font-size: 18px;
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
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.session-meta {
  font-size: 13px;
  color: var(--color-text-tertiary);
  margin-top: 6px;
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
    'Courier New', monospace;
}

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: auto;
  border-top: 1px solid var(--color-bg-tertiary);
  padding-top: 16px;
}

.session-tags {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.mini-tag {
  font-size: 10px;
  background: var(--color-bg-tertiary);
  padding: 2px 8px;
  border-radius: 6px;
  color: var(--color-text-secondary);
  border: 1px solid var(--color-border-primary);
}

.connect-hint {
  font-size: 12px;
  color: var(--color-primary);
  font-weight: 500;
  opacity: 0;
  transition: 0.2s;
  display: flex;
  align-items: center;
  gap: 4px;
}

.session-card:hover .connect-hint {
  opacity: 1;
}

.empty-card {
  border: 2px dashed var(--color-border-primary);
  background: transparent;
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--color-text-tertiary);
  cursor: pointer;
  min-height: 180px;
  transition: 0.2s;
}

.empty-card:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: var(--color-bg-secondary);
}

.empty-card .plus {
  font-size: 32px;
}
</style>
