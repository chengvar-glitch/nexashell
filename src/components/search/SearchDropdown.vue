<template>
  <Teleport to="body">
    <Transition name="search-dropdown-fade">
      <div
        v-show="visible"
        ref="dropdownRef"
        class="search-dropdown panel"
        :style="dropdownStyle"
      >
        <div class="search-dropdown-content">
          <div
            v-if="filteredItems.length === 0 && searchQuery"
            class="search-dropdown-empty"
          >
            <p>{{ $t('search.noMatches') }}</p>
          </div>
          <div v-else class="search-dropdown-list">
            <template v-for="(item, index) in filteredItems" :key="item.id">
              <!-- Section Header -->
              <div
                v-if="item.isHeader"
                class="search-section-header"
                :class="{ 'first-header': index === 0 }"
              >
                {{ item.title }}
              </div>

              <!-- Regular Item -->
              <div
                v-else
                class="search-dropdown-item"
                :class="{
                  active: activeIndex === index,
                }"
                @click="selectItem(item)"
                @mouseenter="activeIndex = index"
              >
                <div class="item-icon">
                  <component :is="item.icon" :size="14" />
                </div>
                <div class="item-content">
                  <div class="item-title">
                    {{ item.title }}
                    <span v-if="item.isRecent" class="recent-badge">{{
                      $t('home.recent')
                    }}</span>
                  </div>
                  <div class="item-description">
                    {{ item.description }}
                  </div>
                </div>
                <div v-if="item.shortcut" class="item-shortcut">
                  <span class="shortcut-text">{{ item.shortcut }}</span>
                </div>
                <div v-else-if="activeIndex === index" class="item-enter-hint">
                  <CornerDownLeft :size="12" />
                </div>
              </div>
            </template>
          </div>
        </div>

        <div class="search-dropdown-footer">
          <div class="navigation-hint">
            <span class="shortcut-text">↑↓/Tab</span> {{ $t('search.select') }}
            <span class="shortcut-text">Enter</span> {{ $t('search.confirm') }}
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Settings,
  Terminal,
  FileText,
  Code,
  Folder,
  Globe,
  Command,
  Server,
  CornerDownLeft,
} from 'lucide-vue-next';
import { sessionApi } from '@/features/session';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';
import type { SavedSession } from '@/features/session/types';

interface SearchItem {
  id: string;
  title: string;
  description: string;
  icon: any;
  shortcut?: string;
  category: string;
  action: () => void;
}

const props = defineProps<{
  visible: boolean;
  anchorElement?: HTMLElement;
  searchQuery?: string;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const { t } = useI18n();

// Use search query passed from parent component, otherwise use local state
const localSearchQuery = ref('');
const searchQuery = computed({
  get: () => props.searchQuery ?? localSearchQuery.value,
  set: value => {
    localSearchQuery.value = value;
  },
});
const activeIndex = ref(-1);
const dropdownRef = ref<HTMLElement | null>(null);
const dropdownStyle = ref({});
const savedSessions = ref<SavedSession[]>([]);

const fetchSavedSessions = async () => {
  try {
    const sessions = await sessionApi.listSessions();
    savedSessions.value = sessions;
  } catch (error) {
    console.error('Failed to fetch saved sessions for search:', error);
  }
};

const updateDropdownPosition = () => {
  if (!props.anchorElement) return;

  const rect = props.anchorElement.getBoundingClientRect();

  dropdownStyle.value = {
    position: 'fixed',
    left: `${rect.left}px`,
    top: `${rect.bottom}px`,
    width: `${rect.width}px`,
    zIndex: 9999,
  };
};

watch(
  () => props.visible,
  newVal => {
    if (newVal) {
      fetchSavedSessions();
      nextTick(() => {
        updateDropdownPosition();
        // Start at index 1 if 0 is a header
        activeIndex.value =
          filteredItems.value.length > 0 && filteredItems.value[0].isHeader
            ? 1
            : 0;
      });
    } else {
      searchQuery.value = '';
      activeIndex.value = -1;
    }
  }
);

watch(
  () => props.anchorElement,
  () => {
    if (props.visible) {
      nextTick(() => {
        updateDropdownPosition();
      });
    }
  }
);

const handleResize = () => {
  if (props.visible) {
    updateDropdownPosition();
  }
};

const searchItems = computed(() => [
  {
    id: 'settings',
    title: t('settings.openSettings'),
    description: t('search.descSettings'),
    icon: Settings,
    shortcut: 'Cmd+,',
    category: 'settings',
    action: () => window.dispatchEvent(new CustomEvent('app:open-settings')),
  },
  {
    id: 'terminal',
    title: t('search.newTerminal'),
    description: t('search.descTerminal'),
    icon: Terminal,
    shortcut: 'Cmd+Shift+T',
    category: 'terminal',
    action: () => window.dispatchEvent(new CustomEvent('app:new-local-tab')),
  },
  {
    id: 'ssh',
    title: t('search.newSSH'),
    description: t('search.descSSH'),
    icon: Server,
    shortcut: 'Cmd+T',
    category: 'terminal',
    action: () => window.dispatchEvent(new CustomEvent('app:open-ssh-form')),
  },
  {
    id: 'help',
    title: t('search.help'),
    description: t('search.descHelp'),
    icon: FileText,
    category: 'help',
    action: () => window.dispatchEvent(new CustomEvent('app:open-help')),
  },
  {
    id: 'command-palette',
    title: t('search.commandPalette'),
    description: t('search.descCommand'),
    icon: Command,
    shortcut: 'Cmd+Shift+P',
    category: 'commands',
    action: () =>
      eventBus.emit(APP_EVENTS.OPEN_SETTINGS, { section: 'shortcuts' }),
  },
  {
    id: 'file-manager',
    title: t('search.fileManager'),
    description: t('search.descFiles'),
    icon: Folder,
    category: 'files',
    action: () =>
      window.dispatchEvent(new CustomEvent('app:open-file-manager')),
  },
  {
    id: 'web-terminal',
    title: t('search.webTerminal'),
    description: t('search.descWeb'),
    icon: Globe,
    category: 'terminal',
    action: () =>
      window.dispatchEvent(new CustomEvent('app:open-web-terminal')),
  },
  {
    id: 'code-editor',
    title: t('search.codeEditor'),
    description: t('search.descEditor'),
    icon: Code,
    category: 'editor',
    action: () => window.dispatchEvent(new CustomEvent('app:open-code-editor')),
  },
]);

const filteredItems = computed(() => {
  const query = searchQuery.value.toLowerCase();
  const results: any[] = [];

  // 1. Process Sessions (Recent or Filtered)
  const sessions = [...savedSessions.value]
    .sort((a, b) => {
      const dateA = new Date(a.updated_at || 0).getTime();
      const dateB = new Date(b.updated_at || 0).getTime();
      return dateB - dateA;
    })
    .filter(
      s =>
        !query ||
        s.server_name.toLowerCase().includes(query) ||
        s.addr.toLowerCase().includes(query) ||
        s.username.toLowerCase().includes(query)
    );

  if (sessions.length > 0) {
    results.push({
      id: 'header-sessions',
      title: query ? t('search.results') : t('search.recent'),
      isHeader: true,
    });

    const sessionList = sessions.slice(0, 10).map(s => ({
      id: `session-${s.id}`,
      title: s.server_name,
      description: `${s.username}@${s.addr}`,
      icon: Server,
      category: 'session',
      isRecent: !query && s.updated_at,
      action: () => eventBus.emit(APP_EVENTS.CONNECT_SESSION, s),
    }));

    results.push(...sessionList);
  }

  // 2. Process Commands (Fallback/Actions)
  const commands = searchItems.value.filter(
    item =>
      !query ||
      item.title.toLowerCase().includes(query) ||
      item.description.toLowerCase().includes(query) ||
      item.category.toLowerCase().includes(query)
  );

  if (commands.length > 0) {
    // Only show "Actions" header if we already have sessions or if there is a query
    if (results.length > 0 || query) {
      results.push({
        id: 'header-commands',
        title: t('search.actions'),
        isHeader: true,
      });
    }

    const commandList = commands.slice(0, 5).map(c => ({
      ...c,
      isRecent: false,
    }));

    results.push(...commandList);
  }

  // Final limit: 12 (Headers don't count much but let's keep it tight)
  // Limit the display to a maximum of 10 content items as per requirements.
  const contentItems = results.filter(i => !i.isHeader);
  if (contentItems.length > 10) {
    // Re-calculating to strictly limit to 10 content items
    // If we have sessions, they take priority
    const finalResults: any[] = [];
    let contentCount = 0;

    for (const item of results) {
      if (item.isHeader) {
        finalResults.push(item);
      } else if (contentCount < 10) {
        finalResults.push(item);
        contentCount++;
      }
    }

    // Remove trailing headers if no items followed
    if (
      finalResults.length > 0 &&
      finalResults[finalResults.length - 1].isHeader
    ) {
      finalResults.pop();
    }

    return finalResults;
  }

  return results;
});

const handleKeyDown = (e: KeyboardEvent) => {
  if (filteredItems.value.length === 0) return;

  const getNextSelectableIndex = (current: number, step: number) => {
    let next = current + step;
    while (
      next >= 0 &&
      next < filteredItems.value.length &&
      filteredItems.value[next].isHeader
    ) {
      next += step;
    }
    return next >= 0 && next < filteredItems.value.length ? next : current;
  };

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault();
      activeIndex.value = getNextSelectableIndex(activeIndex.value, 1);
      scrollToActiveItem();
      break;
    case 'ArrowUp':
      e.preventDefault();
      activeIndex.value = getNextSelectableIndex(activeIndex.value, -1);
      scrollToActiveItem();
      break;
    case 'Enter':
      e.preventDefault();
      if (
        activeIndex.value >= 0 &&
        filteredItems.value[activeIndex.value] &&
        !filteredItems.value[activeIndex.value].isHeader
      ) {
        selectItem(filteredItems.value[activeIndex.value]);
      }
      break;

    case 'Tab':
      e.preventDefault();
      activeIndex.value = getNextSelectableIndex(activeIndex.value, 1);
      scrollToActiveItem();
      break;
  }
};

const handleKeyUp = () => {};

const scrollToActiveItem = () => {
  nextTick(() => {
    if (dropdownRef.value && activeIndex.value >= 0) {
      const item = dropdownRef.value.querySelector(
        `.search-dropdown-item:nth-child(${activeIndex.value + 1})`
      );
      if (item) {
        // Use block: 'nearest' for smoother scrolling without jumping too much
        item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  });
};

const selectItem = (item: SearchItem) => {
  if (item.action) {
    item.action();
  }
  emit('update:visible', false);
  searchQuery.value = '';
  activeIndex.value = -1;
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as Node;
  if (dropdownRef.value && !dropdownRef.value.contains(target)) {
    const parentSearchBox = document.querySelector(
      '.window-title-bar .search-container'
    );
    if (parentSearchBox && parentSearchBox.contains(target)) {
      return;
    }
    emit('update:visible', false);
  }
};

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  window.removeEventListener('resize', handleResize);
});

defineExpose({
  handleKeyDown,
  handleKeyUp,
  updatePosition: updateDropdownPosition,
});
</script>

<style scoped>
.search-dropdown {
  max-height: 420px;
  display: flex;
  flex-direction: column;
  border-radius: 0 0 var(--radius-2xl) var(--radius-2xl);
  overflow: hidden;
  border: 1px solid var(--color-border-secondary);
  border-top: none;
}

.search-dropdown-header {
  padding: 4px 12px 8px;
  border-bottom: 0.5px solid var(--color-border-secondary);
}

.search-section-header {
  padding: 6px 12px 2px;
  font-size: 10px;
  font-weight: 700;
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  opacity: 0.85;
}

:root.theme-light .search-section-header {
  color: var(--color-text-secondary);
  opacity: 1;
}

.search-section-header.first-header {
  padding-top: 2px;
}

.search-dropdown-content {
  flex: 1;
  overflow-y: auto;
  padding: 2px 0;
}

.search-dropdown-list {
  display: flex;
  flex-direction: column;
}

.search-dropdown-item {
  display: flex;
  align-items: center;
  padding: 5px 10px;
  cursor: pointer;
  border-radius: var(--radius-md);
  margin: 0 4px;
  transition:
    background-color 0.15s ease,
    transform 0.1s ease;
  transform: scale(1);
}

.search-dropdown-item.active {
  background-color: var(--color-interactive-selected);
  color: var(--color-text-primary);
  transform: scale(1.005);
}

.search-dropdown-item:active {
  transform: scale(0.99);
}

.item-icon {
  margin-right: 10px;
  color: var(--color-text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
}

.search-dropdown-item.active .item-icon {
  color: var(--color-primary);
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-primary);
  margin-bottom: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.2;
}

.recent-badge {
  font-size: 9px;
  padding: 1px 5px;
  background-color: var(--color-bg-tertiary);
  color: var(--color-text-tertiary);
  border: 1px solid var(--color-border-primary);
  border-radius: 4px;
  font-weight: 600;
  line-height: 1;
  display: inline-flex;
  align-items: center;
}

:root.theme-light .recent-badge {
  background-color: #f1f5f9;
  color: #475569;
  border-color: #cbd5e1;
}

.search-dropdown-item.active .recent-badge {
  background-color: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.3);
  color: white;
}

/* For light themes where active might be primary-colored but light */
:root.theme-light .search-dropdown-item.active .recent-badge {
  background-color: rgba(0, 0, 0, 0.05);
  border-color: rgba(0, 0, 0, 0.1);
  color: inherit;
}

.search-dropdown-item.active .item-title {
  color: var(--color-text-primary);
}

.item-description {
  font-size: 10.5px;
  color: var(--color-text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.2;
}

.search-dropdown-item.active .item-description {
  color: var(--color-text-secondary);
}

.item-shortcut {
  margin-left: 8px;
  flex-shrink: 0;
}

.item-enter-hint {
  margin-left: 8px;
  padding: 3px 5px;
  background-color: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: 4px;
  color: var(--color-text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.search-dropdown-item.active .item-enter-hint {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: white;
  opacity: 1;
  box-shadow: 0 2px 4px rgba(var(--color-primary-rgb), 0.3);
}

:root.theme-light .search-dropdown-item.active .item-enter-hint {
  /* Ensure it pops on light theme accent */
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: white;
}

.shortcut-text {
  display: inline-block;
  padding: 2px 5px;
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  font-size: 10px;
  color: var(--color-text-tertiary);
  font-family:
    ui-monospace, 'SF Mono', 'Cascadia Mono', 'Consolas', Monaco, 'Courier New',
    monospace;
  line-height: 1;
}

.item-shortcut {
  margin-left: 12px;
}

.shortcut-text {
  display: inline-block;
  padding: 3px 7px;
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  font-size: 11px;
  color: var(--color-text-tertiary);
  font-family:
    ui-monospace, 'SF Mono', 'Cascadia Mono', 'Consolas', Monaco, 'Courier New',
    monospace;
  line-height: 1;
}

.search-dropdown-item.active .shortcut-text {
  background-color: var(--color-primary);
  color: white;
}

.search-dropdown-empty {
  padding: 20px;
  text-align: center;
  color: var(--color-text-tertiary);
  font-size: 14px;
}

.search-dropdown-footer {
  padding: 10px 16px;
  border-top: 0.5px solid var(--color-border-secondary);
  background-color: var(--color-bg-tertiary);
  display: flex;
  justify-content: flex-end;
}

.navigation-hint {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.navigation-hint .shortcut-text {
  margin: 0 5px;
  padding: 3px 7px;
  background-color: var(--color-bg-secondary);
  border-radius: var(--radius-sm);
  font-size: 11px;
}

/* Animation effects - macOS style */
.search-dropdown-fade-enter-active,
.search-dropdown-fade-leave-active {
  transition:
    opacity var(--transition-base),
    transform var(--transition-base);
}

.search-dropdown-fade-enter-from {
  opacity: 0;
  transform: scale(0.98);
}

.search-dropdown-fade-leave-to {
  opacity: 0;
  transform: scale(0.98);
}

/* Scrollbar style - hidden by default, shown on hover */
.search-dropdown-content::-webkit-scrollbar {
  width: 6px;
  background: transparent;
}

.search-dropdown-content::-webkit-scrollbar-track {
  background: transparent;
}

.search-dropdown-content::-webkit-scrollbar-thumb {
  background-color: transparent;
  border-radius: var(--radius-sm);
  transition: background-color var(--transition-base);
}

.search-dropdown-content:hover::-webkit-scrollbar-thumb {
  background-color: var(--color-border-primary);
}

.search-dropdown-content::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-text-tertiary);
}
</style>
