<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, inject } from 'vue';
import TabItem from '@/components/common/TabItem.vue';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ShortcutHint from '@/components/common/ShortcutHint.vue';
import { Plus, ChevronDown, X, MoreHorizontal } from 'lucide-vue-next';
import { TAB_MANAGEMENT_KEY, OPEN_SSH_FORM_KEY } from '@/core/types';
import { NEW_TAB_MENU_ITEMS } from '@/core/constants';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';

// Inject tab management functionality
const tabManagement = inject(TAB_MANAGEMENT_KEY);
if (!tabManagement) {
  throw new Error('tabManagement not provided');
}
const tabs = tabManagement.tabs;
const activeTabId = tabManagement.activeTabId;

// Inject SSH form control method
const openSSHForm = inject(OPEN_SSH_FORM_KEY);
if (!openSSHForm) {
  console.warn('[AppTabs] openSSHForm not provided by parent component');
}

const isDropdownOpen = ref(false);
const dropdownX = ref(0);
const dropdownY = ref(0);
let tabCounter = 1;

const tabsContainerRef = ref<HTMLElement>();

// Use constant definitions
// const NEW_TAB_MENU = NEW_TAB_MENU_ITEMS;

const handleTabClick = async (id: string) => {
  tabManagement.setActiveTab(id);

  await nextTick();
  scrollToActiveTab();
};

const handleTabClose = async (id: string) => {
  await tabManagement.closeTab(id);
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
    // Open SSH form modal instead of creating a tab
    if (openSSHForm) {
      openSSHForm();
    }
  }
  isDropdownOpen.value = false;

  await nextTick();
  scrollToActiveTab();
};

const handleCloseTabShortcut = () => {
  const currentTab = tabs.value.find(tab => tab.id === activeTabId.value);
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
  // Open SSH form modal instead of creating a tab
  if (openSSHForm) {
    openSSHForm();
  }
};

const handleNewTabShortcut = () => {
  handleAddTab();
};

// Open SSH connection form when clicking the plus button
const openSSHConnectionForm = () => {
  // Open SSH form modal
  if (openSSHForm) {
    openSSHForm();
  }
};

// Scroll to the currently active tab
const scrollToActiveTab = () => {
  if (!tabsContainerRef.value) return;

  const activeTabElement = document.querySelector(
    `.tab-item[data-id="${activeTabId.value}"]`
  ) as HTMLElement;
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
      newScrollLeft =
        containerScrollLeft + (tabOffsetLeft + tabWidth - containerWidth);
    }

    // Scroll to the target position
    tabsContainerRef.value.scrollTo({
      left: newScrollLeft,
      behavior: 'smooth',
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
  <div class="app-tabs glass-light border-bottom" data-tauri-drag-region>
    <div
      ref="tabsContainerRef"
      class="tabs-container scrollbar-hidden"
      data-tauri-drag-region
    >
      <TransitionGroup name="tab-list">
        <TabItem
          v-for="tab in tabs"
          :id="tab.id"
          :key="tab.id"
          :label="tab.label"
          :type="tab.type"
          :active="tab.id === activeTabId"
          :closable="tab.closable"
          :data-id="tab.id"
          @click="handleTabClick"
          @close="handleTabClose"
        />
      </TransitionGroup>

      <div class="tab-actions" :class="{ 'is-active': isDropdownOpen }">
        <ShortcutHint text="Cmd+T to create SSH connection" position="bottom">
          <button
            class="action-btn"
            :class="{ 'is-active': isDropdownOpen }"
            aria-label="Add SSH connection"
            @click="openSSHConnectionForm"
          >
            <Plus :size="14" />
          </button>
        </ShortcutHint>
        <ShortcutHint text="More options" position="bottom">
          <button
            class="action-btn dropdown-btn"
            :class="{ 'is-active': isDropdownOpen }"
            aria-label="More options"
            @click="toggleDropdown"
          >
            <ChevronDown v-if="!isDropdownOpen" :size="14" />
            <X v-else :size="14" />
          </button>
        </ShortcutHint>
      </div>
    </div>

    <div class="right-actions" data-tauri-drag-region>
      <ShortcutHint text="Window Actions" position="bottom">
        <button class="action-btn more-btn" aria-label="More">
          <MoreHorizontal :size="16" />
        </button>
      </ShortcutHint>
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
  display: flex;
  align-items: center;
  background-color: var(--color-bg-secondary);
  padding: 0 4px;
  height: 36px;
  overflow: hidden;
  position: relative;
}

.tabs-container {
  display: flex;
  align-items: center;
  height: 100%;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 0 4px;
}

/* Hide scrollbar but keep scrolling functionality */
.tabs-container::-webkit-scrollbar {
  display: none;
}

.tabs-container {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

/* Tab animations */
.tab-list-enter-active,
.tab-list-leave-active {
  transition: all 0.3s ease;
}
.tab-list-enter-from {
  opacity: 0;
  transform: translateY(10px);
}
.tab-list-leave-to {
  opacity: 0;
  transform: scale(0.9);
}
.tab-list-move {
  transition: transform 0.3s ease;
}

.tab-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-left: 8px;
  padding: 2px;
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  flex-shrink: 0;
}

.tab-actions.is-active {
  background-color: var(--color-interactive-hover);
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background-color: transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all var(--transition-base);
}

.action-btn:hover {
  background-color: var(--color-bg-hover);
  color: var(--color-text-primary);
}

.right-actions {
  display: flex;
  align-items: center;
  padding: 0 8px;
  margin-left: auto;
  border-left: 1px solid var(--color-border-tertiary);
}

.more-btn {
  color: var(--color-text-tertiary);
}

.dropdown-btn {
  border-radius: var(--radius-md);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .action-btn:hover {
    background-color: var(--color-bg-hover);
  }
}
</style>
