<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, inject } from 'vue';
import TabItem from '@/components/common/TabItem.vue';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ShortcutHint from '@/components/common/ShortcutHint.vue';
import { Plus, ChevronDown, X, MoreHorizontal } from 'lucide-vue-next';
import { TAB_MANAGEMENT_KEY, OPEN_SSH_FORM_KEY } from '@/types';
import { NEW_TAB_MENU_ITEMS } from '@/constants';
import { APP_EVENTS } from '@/constants';
import { eventBus } from '@/utils/event-bus';

// 注入标签管理功能
const tabManagement = inject(TAB_MANAGEMENT_KEY);
if (!tabManagement) {
  throw new Error('tabManagement not provided');
}
const tabs = tabManagement.tabs;
const activeTabId = tabManagement.activeTabId;

// 注入SSH表单控制方法
const openSSHForm = inject(OPEN_SSH_FORM_KEY);

const isDropdownOpen = ref(false);
const dropdownX = ref(0);
const dropdownY = ref(0);
let tabCounter = 1;

const tabsContainerRef = ref<HTMLDivElement>();

// 使用常量定义
// const NEW_TAB_MENU = NEW_TAB_MENU_ITEMS;

const handleTabClick = async (id: string) => {
  tabManagement.setActiveTab(id);
  
  await nextTick();
  scrollToActiveTab();
};

const handleTabClose = (id: string) => {
  tabManagement.closeTab(id);
};

const handleAddTab = async () => {
  const newTab = {
    id: `tab-${Date.now()}-${tabCounter++}`,
    label: `Terminal ${tabCounter}`,
    type: 'terminal' as const,
    closable: true,
  };
  tabManagement.addTab(newTab);
  
  await nextTick();
  scrollToActiveTab();
};

const toggleDropdown = (event: MouseEvent) => {
  event.stopPropagation();
  if (!isDropdownOpen.value) {
    const target = event.currentTarget as HTMLElement;
    const container = target.closest('.tab-actions') as HTMLElement;
    if (container) {
      const rect = container.getBoundingClientRect();
      
      // Calculate the available space on the right side
      const availableRightSpace = window.innerWidth - rect.left;
      const menuWidth = 200; // Approximate dropdown menu width
      
      // Adjust x position if menu would go off-screen
      if (availableRightSpace < menuWidth) {
        // Position the menu to appear from the right edge of the button
        dropdownX.value = Math.max(rect.right - menuWidth, 0); // Ensure it doesn't go off the left edge
      } else {
        dropdownX.value = rect.left;
      }
      
      dropdownY.value = rect.bottom + 2;
    }
  }
  isDropdownOpen.value = !isDropdownOpen.value;
};

const handleMenuSelect = async (key: string) => {
  if (key === 'local') {
    const newTab = {
      id: `tab-${Date.now()}-${tabCounter++}`,
      label: `Local Terminal ${tabCounter}`,
      type: 'terminal' as const,
      closable: true,
    };
    tabManagement.addTab(newTab);
  } else if (key === 'ssh') {
    // 打开SSH表单弹窗而不是创建标签页
    if (openSSHForm) {
      openSSHForm();
    }
  }
  isDropdownOpen.value = false;
  
  await nextTick();
  scrollToActiveTab();
};

const handleCloseTabShortcut = () => {
  const currentTab = tabs.value.find((tab: any) => tab.id === activeTabId.value);
  if (currentTab && currentTab.closable) {
    handleTabClose(activeTabId.value);
  }
};

const handleNewLocalTab = async () => {
  const newTab = {
    id: `tab-${Date.now()}-${tabCounter++}`,
    label: `Local Terminal ${tabCounter}`,
    type: 'terminal' as const,
    closable: true,
  };
  tabManagement.addTab(newTab);
  
  await nextTick();
  scrollToActiveTab();
};

const handleNewSSHTab = async () => {
  // 打开SSH表单弹窗而不是创建标签页
  if (openSSHForm) {
    openSSHForm();
  }
};

const handleNewTabShortcut = () => {
  handleAddTab();
};

// Open SSH connection form when clicking the plus button
const openSSHConnectionForm = () => {
  // 打开SSH表单弹窗
  if (openSSHForm) {
    openSSHForm();
  }
};

// Scroll to the currently active tab
const scrollToActiveTab = () => {
  if (!tabsContainerRef.value) return;
  
  const activeTabElement = document.querySelector(`.tab-item[data-id="${activeTabId.value}"]`) as HTMLElement;
  if (activeTabElement && tabsContainerRef.value) {
    const containerScrollLeft = tabsContainerRef.value.scrollLeft;
    const containerWidth = tabsContainerRef.value.clientWidth;
    const tabOffsetLeft = activeTabElement.offsetLeft;
    const tabWidth = activeTabElement.offsetWidth;
    
    let newScrollLeft = containerScrollLeft;
    
    // If the tab is outside the view to the left
    if (tabOffsetLeft < 0) {
      newScrollLeft = containerScrollLeft + tabOffsetLeft;
    } 
    // If the tab is outside the view to the right
    else if (tabOffsetLeft + tabWidth > containerWidth) {
      newScrollLeft = containerScrollLeft + (tabOffsetLeft + tabWidth - containerWidth);
    }
    
    // Scroll to the target position
    tabsContainerRef.value.scrollTo({
      left: newScrollLeft,
      behavior: 'smooth'
    });
  }
};

onMounted(() => {
  eventBus.on(APP_EVENTS.CLOSE_TAB, handleCloseTabShortcut);
  eventBus.on(APP_EVENTS.NEW_TAB, handleNewTabShortcut);
  eventBus.on(APP_EVENTS.NEW_LOCAL_TAB, handleNewLocalTab);
  eventBus.on(APP_EVENTS.NEW_SSH_TAB, handleNewSSHTab);
  
  window.addEventListener('resize', scrollToActiveTab);
});

onBeforeUnmount(() => {
  eventBus.off(APP_EVENTS.CLOSE_TAB, handleCloseTabShortcut);
  eventBus.off(APP_EVENTS.NEW_TAB, handleNewTabShortcut);
  eventBus.off(APP_EVENTS.NEW_LOCAL_TAB, handleNewLocalTab);
  eventBus.off(APP_EVENTS.NEW_SSH_TAB, handleNewSSHTab);
  
  window.removeEventListener('resize', scrollToActiveTab);
});
</script>

<template>
  <div class="app-tabs glass-light border-bottom">
    <div
      ref="tabsContainerRef"
      class="tabs-container scrollbar-hidden"
    >
      <TabItem
        v-for="(tab, index) in tabs"
        :id="tab.id"
        :key="tab.id"
        :label="tab.label"
        :active="tab.id === activeTabId"
        :closable="tab.closable"
        :class="{ 'fixed-tab': index === 0 }"
        :data-id="tab.id"
        @click="handleTabClick"
        @close="handleTabClose"
      />
      <div
        class="tab-actions"
        :class="{ 'is-active': isDropdownOpen }"
      >
        <ShortcutHint
          text="Cmd+T to create SSH connection"
          position="bottom"
        >
          <button
            class="action-btn"
            :class="{ 'is-active': isDropdownOpen }"
            aria-label="Add SSH connection"
            @click="openSSHConnectionForm"
          >
            <Plus :size="14" />
          </button>
        </ShortcutHint>
        <ShortcutHint
          text="More options"
          position="bottom"
        >
          <button
            class="action-btn"
            :class="{ 'is-active': isDropdownOpen }"
            aria-label="More options"
            @click="toggleDropdown"
          >
            <ChevronDown
              v-if="!isDropdownOpen"
              :size="14"
            />
            <X
              v-else
              :size="14"
            />
          </button>
        </ShortcutHint>
      </div>
    </div>

    <div class="more-container">
      <button
        class="action-btn"
        aria-label="More"
      >
        <MoreHorizontal :size="14" />
      </button>
    </div>

    <DropdownMenu
      v-model:visible="isDropdownOpen"
      :items="NEW_TAB_MENU_ITEMS"
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

/* Hide scrollbar but keep scrolling functionality */
.tabs-container::-webkit-scrollbar {
  display: none;
}

.tabs-container {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

/* Fixed style for the first tab */
.fixed-tab {
  flex-shrink: 0;
  position: sticky;
  left: 0;
  z-index: 10;
  background-color: var(--color-bg-secondary);
  margin-left: 0;
  border-left: none;
  border-radius: 0 var(--radius-lg) var(--radius-lg) 0;
  margin-right: 0;
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