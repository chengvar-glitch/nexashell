<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';
import TabItem from '../ui/TabItem.vue';
import DropdownMenu from '../ui/DropdownMenu.vue';
import ShortcutHint from '../ui/ShortcutHint.vue';
import { Plus, ChevronDown, X, MoreHorizontal } from 'lucide-vue-next';

interface Tab {
  id: string;
  label: string;
  closable: boolean;
}

const tabs = ref<Tab[]>([
  { id: 'nexashell-default', label: 'NEXASHELL', closable: false }
]);

const activeTabId = ref('nexashell-default');
const isDropdownOpen = ref(false);
const dropdownX = ref(0);
const dropdownY = ref(0);
let tabCounter = 1;

const NEW_TAB_MENU = [
  { key: 'local', label: 'Local Terminal', shortcut: 'Cmd+Shift+T' },
  { key: 'ssh', label: 'Remote Connection', shortcut: 'Cmd+T' },
];

const handleTabClick = (id: string) => {
  activeTabId.value = id;
};

const handleTabClose = (id: string) => {
  const index = tabs.value.findIndex(tab => tab.id === id);
  if (index === -1) return;

  tabs.value.splice(index, 1);

  if (id === activeTabId.value && tabs.value.length > 0) {
    const newIndex = Math.min(index, tabs.value.length - 1);
    activeTabId.value = tabs.value[newIndex].id;
  }
};

const handleAddTab = () => {
  const newTab: Tab = {
    id: `tab-${Date.now()}-${tabCounter++}`,
    label: `Terminal ${tabCounter}`,
    closable: true
  };
  tabs.value.push(newTab);
  activeTabId.value = newTab.id;
};

const toggleDropdown = (event: MouseEvent) => {
  event.stopPropagation();
  if (!isDropdownOpen.value) {
    const target = event.currentTarget as HTMLElement;
    const container = target.closest('.tab-actions') as HTMLElement;
    if (container) {
      const rect = container.getBoundingClientRect();
      dropdownX.value = rect.left;
      dropdownY.value = rect.bottom + 2;
    }
  }
  isDropdownOpen.value = !isDropdownOpen.value;
};

const handleMenuSelect = (key: string) => {
  if (key === 'local') {
    const newTab: Tab = {
      id: `tab-${Date.now()}-${tabCounter++}`,
      label: `Local Terminal ${tabCounter}`,
      closable: true
    };
    tabs.value.push(newTab);
    activeTabId.value = newTab.id;
  } else if (key === 'ssh') {
    const newTab: Tab = {
      id: `tab-${Date.now()}-${tabCounter++}`,
      label: `SSH ${tabCounter}`,
      closable: true
    };
    tabs.value.push(newTab);
    activeTabId.value = newTab.id;
  }
  isDropdownOpen.value = false;
};

const handleCloseTabShortcut = () => {
  const currentTab = tabs.value.find(tab => tab.id === activeTabId.value);
  if (currentTab && currentTab.closable) {
    handleTabClose(activeTabId.value);
  }
};

const handleNewLocalTab = () => {
  const newTab: Tab = {
    id: `tab-${Date.now()}-${tabCounter++}`,
    label: `Local Terminal ${tabCounter}`,
    closable: true
  };
  tabs.value.push(newTab);
  activeTabId.value = newTab.id;
};

const handleNewSSHTab = () => {
  const newTab: Tab = {
    id: `tab-${Date.now()}-${tabCounter++}`,
    label: `SSH ${tabCounter}`,
    closable: true
  };
  tabs.value.push(newTab);
  activeTabId.value = newTab.id;
};

const handleNewTabShortcut = () => {
  handleAddTab();
};

onMounted(() => {
  window.addEventListener('app:close-tab', handleCloseTabShortcut);
  window.addEventListener('app:new-tab', handleNewTabShortcut);
  window.addEventListener('app:new-local-tab', handleNewLocalTab);
  window.addEventListener('app:new-ssh-tab', handleNewSSHTab);
});

onBeforeUnmount(() => {
  window.removeEventListener('app:close-tab', handleCloseTabShortcut);
  window.removeEventListener('app:new-tab', handleNewTabShortcut);
  window.removeEventListener('app:new-local-tab', handleNewLocalTab);
  window.removeEventListener('app:new-ssh-tab', handleNewSSHTab);
});
</script>

<template>
  <div class="app-tabs glass-light border-bottom">
    <div class="tabs-container scrollbar-hidden">
      <TabItem
        v-for="tab in tabs"
        :key="tab.id"
        :id="tab.id"
        :label="tab.label"
        :active="tab.id === activeTabId"
        :closable="tab.closable"
        @click="handleTabClick"
        @close="handleTabClose"
      />
      <div class="tab-actions" :class="{ 'is-active': isDropdownOpen }">
        <ShortcutHint text="Cmd+Shift+T" position="bottom">
          <button class="action-btn" :class="{ 'is-active': isDropdownOpen }" @click="handleAddTab" aria-label="Add tab">
            <Plus :size="14" />
          </button>
        </ShortcutHint>
        <ShortcutHint text="More options" position="bottom">
          <button class="action-btn" :class="{ 'is-active': isDropdownOpen }" @click="toggleDropdown" aria-label="More options">
            <ChevronDown v-if="!isDropdownOpen" :size="14" />
            <X v-else :size="14" />
          </button>
        </ShortcutHint>
      </div>
    </div>
    
    <div class="more-container">
      <button class="action-btn" aria-label="More">
        <MoreHorizontal :size="14" />
      </button>
    </div>
    
    <DropdownMenu
      v-model:visible="isDropdownOpen"
      :items="NEW_TAB_MENU"
      :x="dropdownX"
      :y="dropdownY"
      @select="handleMenuSelect"
    />
  </div>
</template>

<style scoped>
.app-tabs {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  background-color: var(--color-bg-secondary);
  padding: 0;
  height: 44px;
  overflow: hidden;
}

.tabs-container {
  display: flex;
  height: 100%;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
}

.tab-actions {
  display: flex;
  align-items: center;
  gap: 0;
  height: calc(100% - 12px);
  margin-top: 6px;
  margin-bottom: 6px;
  padding: 0 4px;
  flex-shrink: 0;
  background-color: transparent;
  border-radius: var(--radius-md);
  transition: all var(--transition-base);
}

.tab-actions.is-active {
  background-color: var(--color-interactive-hover);
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  margin: 0;
  border: none;
  background-color: transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all var(--transition-base);
}

.action-btn:hover {
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.action-btn.is-active {
  background-color: transparent;
}

.more-container {
  display: flex;
  justify-content: flex-end;
  padding: 0 16px;
}

.more-container .action-btn {
  padding: 6px;
  border-radius: var(--radius-md);
  border: none;
  background-color: transparent;
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all var(--transition-base);
  display: flex;
  align-items: center;
  justify-content: center;
}

.more-container .action-btn:hover {
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .action-btn:hover {
    background-color: var(--color-interactive-hover);
  }
}

:root.theme-dark .action-btn:hover {
  background-color: var(--color-interactive-hover);
}
</style>