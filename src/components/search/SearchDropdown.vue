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
            <p>No matches found</p>
          </div>
          <div
            v-else
            class="search-dropdown-list"
          >
            <div
              v-for="(item, index) in filteredItems"
              :key="item.id"
              class="search-dropdown-item"
              :class="{
                active: activeIndex === index,
                hover:
                  lastInteractionType !== 'keyboard' && hoverIndex === index,
              }"
              @click="selectItem(item)"
              @mouseenter="
                () => {
                  if (lastInteractionType !== 'keyboard') {
                    hoverIndex = index;
                  }
                }
              "
              @mouseleave="
                () => {
                  if (lastInteractionType !== 'keyboard') {
                    hoverIndex = -1;
                  }
                }
              "
            >
              <div class="item-icon">
                <component
                  :is="item.icon"
                  :size="16"
                />
              </div>
              <div class="item-content">
                <div class="item-title">
                  {{ item.title }}
                </div>
                <div class="item-description">
                  {{ item.description }}
                </div>
              </div>
              <div
                v-if="item.shortcut"
                class="item-shortcut"
              >
                <span class="shortcut-text">{{ item.shortcut }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="search-dropdown-footer">
          <div class="navigation-hint">
            <span class="shortcut-text">↑↓/Tab</span> Select
            <span class="shortcut-text">Enter</span> Confirm
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import {
  Settings,
  Terminal,
  FileText,
  Code,
  Folder,
  Globe,
  Command,
} from 'lucide-vue-next';

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

// Use search query passed from parent component, otherwise use local state
const localSearchQuery = ref('');
const searchQuery = computed({
  get: () => props.searchQuery ?? localSearchQuery.value,
  set: value => {
    localSearchQuery.value = value;
  },
});
const activeIndex = ref(-1);
const hoverIndex = ref(-1);
const lastInteractionType = ref<'keyboard' | 'mouse'>('mouse');
const dropdownRef = ref<HTMLElement | null>(null);
const dropdownStyle = ref({});

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
      nextTick(() => {
        updateDropdownPosition();
        activeIndex.value = 0;
        lastInteractionType.value = 'keyboard';
      });
    } else {
      searchQuery.value = '';
      activeIndex.value = -1;
      hoverIndex.value = -1;
      lastInteractionType.value = 'mouse';
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

const searchItems = ref([
  {
    id: 'settings',
    title: 'Open Settings',
    description: 'Configure application preferences',
    icon: Settings,
    shortcut: 'Cmd+,',
    category: 'settings',
    action: () => window.dispatchEvent(new CustomEvent('app:open-settings')),
  },
  {
    id: 'terminal',
    title: 'New Terminal Tab',
    description: 'Create a new terminal session',
    icon: Terminal,
    shortcut: 'Cmd+T',
    category: 'terminal',
    action: () => window.dispatchEvent(new CustomEvent('app:new-local-tab')),
  },
  {
    id: 'ssh',
    title: 'SSH Connection',
    description: 'Connect to a remote server',
    icon: Terminal,
    shortcut: 'Cmd+Shift+T',
    category: 'terminal',
    action: () => window.dispatchEvent(new CustomEvent('app:open-ssh-form')),
  },
  {
    id: 'help',
    title: 'Help Documentation',
    description: 'View help and documentation',
    icon: FileText,
    category: 'help',
    action: () => window.dispatchEvent(new CustomEvent('app:open-help')),
  },
  {
    id: 'command-palette',
    title: 'Command Palette',
    description: 'Quick access to all commands',
    icon: Command,
    shortcut: 'Cmd+Shift+P',
    category: 'commands',
    action: () =>
      window.dispatchEvent(new CustomEvent('app:open-command-palette')),
  },
  {
    id: 'file-manager',
    title: 'File Manager',
    description: 'Browse and manage files',
    icon: Folder,
    category: 'files',
    action: () =>
      window.dispatchEvent(new CustomEvent('app:open-file-manager')),
  },
  {
    id: 'web-terminal',
    title: 'Web Terminal',
    description: 'Use web-based terminal',
    icon: Globe,
    category: 'terminal',
    action: () =>
      window.dispatchEvent(new CustomEvent('app:open-web-terminal')),
  },
  {
    id: 'code-editor',
    title: 'Code Editor',
    description: 'Built-in code editing feature',
    icon: Code,
    category: 'editor',
    action: () => window.dispatchEvent(new CustomEvent('app:open-code-editor')),
  },
] as SearchItem[]);

const filteredItems = computed(() => {
  if (!searchQuery.value) {
    return searchItems.value;
  }
  const query = searchQuery.value.toLowerCase();
  return searchItems.value.filter(
    item =>
      item.title.toLowerCase().includes(query) ||
      item.description.toLowerCase().includes(query) ||
      item.category.toLowerCase().includes(query)
  );
});

const handleKeyDown = (e: KeyboardEvent) => {
  if (filteredItems.value.length === 0) return;

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault();
      activeIndex.value = Math.min(
        activeIndex.value + 1,
        filteredItems.value.length - 1
      );
      lastInteractionType.value = 'keyboard';
      scrollToActiveItem();
      break;
    case 'ArrowUp':
      e.preventDefault();
      activeIndex.value = Math.max(activeIndex.value - 1, 0);
      lastInteractionType.value = 'keyboard';
      scrollToActiveItem();
      break;
    case 'Enter':
      e.preventDefault();
      if (activeIndex.value >= 0 && filteredItems.value[activeIndex.value]) {
        selectItem(filteredItems.value[activeIndex.value]);
      }
      break;

    case 'Tab':
      e.preventDefault();
      activeIndex.value = Math.min(
        activeIndex.value + 1,
        filteredItems.value.length - 1
      );
      lastInteractionType.value = 'keyboard';
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
        item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  });
};

const selectItem = (item: SearchItem) => {
  lastInteractionType.value = 'mouse';
  item.action();
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

.search-dropdown-content {
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
}

.search-dropdown-list {
  display: flex;
  flex-direction: column;
}

.search-dropdown-item {
  display: flex;
  align-items: center;
  padding: 12px 18px;
  cursor: pointer;
  transition: all var(--transition-base);
  border-radius: var(--radius-md);
  margin: 2px 10px;
}

.search-dropdown-item.active {
  background-color: var(--color-interactive-selected);
  color: var(--color-text-primary);
}

.search-dropdown-item.hover:not(.active) {
  background-color: var(--color-interactive-hover);
}

.search-dropdown-item.active.hover {
  background-color: var(--color-interactive-selected);
}

.item-icon {
  margin-right: 14px;
  color: var(--color-text-tertiary);
}

.search-dropdown-item.active .item-icon {
  color: var(--color-primary);
}

.item-content {
  flex: 1;
}

.item-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
  margin-bottom: 3px;
}

.search-dropdown-item.active .item-title {
  color: var(--color-text-primary);
}

.item-description {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.search-dropdown-item.active .item-description {
  color: var(--color-text-secondary);
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
