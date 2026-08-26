<script setup lang="ts">
import {
  ref,
  onMounted,
  onBeforeUnmount,
  nextTick,
  inject,
  computed,
} from 'vue';
import { useI18n } from 'vue-i18n';
import TabItem from '@/components/common/TabItem.vue';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ShortcutHint from '@/components/common/ShortcutHint.vue';
import {
  Plus,
  ChevronDown,
  MoreHorizontal,
  Terminal,
  Server,
} from 'lucide-vue-next';
import { TAB_MANAGEMENT_KEY, OPEN_SSH_FORM_KEY } from '@/core/types';
import { NEW_TAB_MENU_ITEMS } from '@/core/constants';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';
import { sortByUpdatedAtDesc } from '@/core/utils/time-utils';
import { formatShortcut } from '@/core/utils/platform/platform-detection';
import { createLogger } from '@/core/utils/logger';
import { sessionApi } from '@/features/session';
import type { SavedSession } from '@/features/session';

const logger = createLogger('APP_TABS');

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
  logger.warn('openSSHForm not provided by parent component');
}

const { t } = useI18n({ useScope: 'global' });

const isDropdownOpen = ref(false);
const dropdownX = ref(0);
const dropdownY = ref(0);
let tabCounter = 1;

// Saved connections dropdown (shown by the "+" button)
const savedConnectionsOpen = ref(false);
const savedConnectionsX = ref(0);
const savedConnectionsY = ref(0);
const savedConnections = ref<SavedSession[]>([]);
// TTL cache: avoid re-querying the DB via IPC on every toggle of the
// dropdown. The list only changes on save/edit/delete, which bump the
// timestamp; refetching after this long is cheap insurance.
const SAVED_CONNECTIONS_CACHE_TTL = 30_000;
let savedConnectionsLoadedAt = 0;

// Compute a dropdown position just below `event`'s element, clamping x so the
// menu does not run off the right edge. Shared by both dropdowns.
const positionMenuBelow = (
  event: MouseEvent,
  menuWidth: number
): { x: number; y: number } => {
  const target = event.currentTarget as HTMLElement;
  const container = target.closest('.tab-actions') as HTMLElement;
  if (!container) {
    return { x: 0, y: 0 };
  }
  const rect = container.getBoundingClientRect();
  const availableRightSpace = window.innerWidth - rect.left;
  const x =
    availableRightSpace < menuWidth
      ? Math.max(rect.right - menuWidth, 0)
      : rect.left;
  return { x, y: rect.bottom + 2 };
};

const tabsContainerRef = ref<HTMLElement>();

// Translate menu items reactively
const translatedMenuItems = computed(() =>
  NEW_TAB_MENU_ITEMS.map(item => ({
    ...item,
    label: t(item.label),
    shortcut: item.shortcut ? formatShortcut(item.shortcut) : undefined,
    icon: item.key === 'local' ? Terminal : Server,
  }))
);

const handleTabClick = async (id: string) => {
  tabManagement.setActiveTab(id);

  await nextTick();
  scrollToActiveTab();
};

const handleTabClose = async (id: string) => {
  await tabManagement.closeTab(id);
};

const toggleDropdown = (event: MouseEvent) => {
  event.stopPropagation();
  if (!isDropdownOpen.value) {
    const { x, y } = positionMenuBelow(event, 200);
    dropdownX.value = x;
    dropdownY.value = y;
    savedConnectionsOpen.value = false;
  }
  isDropdownOpen.value = !isDropdownOpen.value;
};

const createLocalTab = async () => {
  const currentCounter = tabCounter++;
  const newTab = {
    id: crypto.randomUUID(),
    label: `${t('settings.newLocalTab')} ${currentCounter}`,
    type: 'terminal' as const,
    closable: true,
    panes: [{ id: crypto.randomUUID(), type: 'terminal' as const }],
  };
  tabManagement.addTab(newTab);

  await nextTick();
  scrollToActiveTab();
};

const handleMenuSelect = async (key: string) => {
  if (key === 'local') {
    await createLocalTab();
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
  if (!currentTab || !currentTab.closable) return;

  // If tab has multiple panes, close the active pane first
  if (currentTab.panes && currentTab.panes.length > 1) {
    const paneId = tabManagement.activePaneId.value;
    if (paneId) {
      tabManagement.closePane(currentTab.id, paneId);
      return;
    }
  }

  handleTabClose(activeTabId.value);
};

const handleNewLocalTab = async () => {
  await createLocalTab();
};

const handleNewSSHTab = async () => {
  // Open SSH form modal instead of creating a tab
  if (openSSHForm) {
    openSSHForm();
  }
};

// Saved connections dropdown (shown by the "+" button)
const translatedSavedConnections = computed<Array<{
  key: string;
  label: string;
  icon: typeof Server;
  disabled?: boolean;
  divider?: boolean;
}>>(() => {
  const items: Array<{
    key: string;
    label: string;
    icon: typeof Server;
    disabled?: boolean;
    divider?: boolean;
  }> = [
    {
      key: '__new__',
      label: t('tabs.newConnection'),
      icon: Plus,
    },
  ];
  if (savedConnections.value.length === 0) {
    items.push(
      {
        key: '__divider__',
        label: '—',
        icon: Server,
        divider: true,
      },
      {
        key: '__empty__',
        label: t('tabs.noSavedConnections'),
        icon: Server,
        disabled: true,
      }
    );
  } else {
    items.push({
      key: '__divider__',
      label: '—',
      icon: Server,
      divider: true,
    });
    for (const session of savedConnections.value) {
      items.push({
        key: session.id,
        label: session.server_name,
        icon: Server,
      });
    }
  }
  return items;
});

const sortSavedConnections = (list: SavedSession[]): SavedSession[] => {
  return sortByUpdatedAtDesc(list);
};

const toggleSavedConnections = async (event: MouseEvent) => {
  event.stopPropagation();
  if (!savedConnectionsOpen.value) {
    const { x, y } = positionMenuBelow(event, 220);
    savedConnectionsX.value = x;
    savedConnectionsY.value = y;
    isDropdownOpen.value = false;

    // Reuse the cached list within the TTL window to avoid an IPC + DB round
    // trip on every open. Sessions are shown newest-first.
    const now = Date.now();
    if (
      savedConnections.value.length === 0 ||
      now - savedConnectionsLoadedAt > SAVED_CONNECTIONS_CACHE_TTL
    ) {
      try {
        const list = await sessionApi.listSessions();
        savedConnections.value = sortSavedConnections(list);
        savedConnectionsLoadedAt = now;
      } catch (err) {
        logger.warn('Failed to load saved connections', err);
        // Keep whatever is cached on a transient failure.
      }
    }
  }
  savedConnectionsOpen.value = !savedConnectionsOpen.value;
};

// Invalidate the saved-connections cache when a session is saved/edited so
// the next open pulls fresh data.
const invalidateSavedConnectionsCache = () => {
  savedConnectionsLoadedAt = 0;
};

const handleSavedSessionSelect = (key: string) => {
  savedConnectionsOpen.value = false;
  if (key === '__empty__' || key === '__divider__') return;
  if (key === '__new__') {
    if (openSSHForm) {
      openSSHForm();
    }
    return;
  }
  const session = savedConnections.value.find(s => s.id === key);
  if (!session) return;
  eventBus.emit(APP_EVENTS.CONNECT_SESSION, session);
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
  eventBus.on(APP_EVENTS.NEW_LOCAL_TAB, handleNewLocalTab);
  eventBus.on(APP_EVENTS.NEW_SSH_TAB, handleNewSSHTab);
  eventBus.on(APP_EVENTS.SESSION_SAVED, invalidateSavedConnectionsCache);

  window.addEventListener('resize', scrollToActiveTab);
});

onBeforeUnmount(() => {
  eventBus.off(APP_EVENTS.CLOSE_TAB, handleCloseTabShortcut);
  eventBus.off(APP_EVENTS.NEW_LOCAL_TAB, handleNewLocalTab);
  eventBus.off(APP_EVENTS.NEW_SSH_TAB, handleNewSSHTab);
  eventBus.off(APP_EVENTS.SESSION_SAVED, invalidateSavedConnectionsCache);

  window.removeEventListener('resize', scrollToActiveTab);
});
</script>

<template>
  <div class="app-tabs border-bottom" data-tauri-drag-region>
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

      <div
        class="tab-actions"
        :class="{ 'is-active': isDropdownOpen || savedConnectionsOpen }"
      >
        <ShortcutHint
          :text="t('tabs.savedConnections')"
          position="bottom"
        >
          <button
            class="action-btn"
            :class="{ 'is-active': savedConnectionsOpen }"
            :aria-label="t('tabs.savedConnections')"
            @click="toggleSavedConnections"
          >
            <Plus :size="14" />
          </button>
        </ShortcutHint>
        <ShortcutHint :text="t('common.moreOptions')" position="bottom">
          <button
            class="action-btn dropdown-btn"
            :class="{ 'is-active': isDropdownOpen }"
            :aria-label="t('common.moreOptions')"
            @click="toggleDropdown"
          >
          <ChevronDown :size="14" />
          </button>
        </ShortcutHint>
      </div>
    </div>

    <div class="right-actions" data-tauri-drag-region>
      <ShortcutHint :text="t('common.windowActions')" position="bottom">
        <button
          class="action-btn more-btn"
          :aria-label="t('common.windowActions')"
        >
          <MoreHorizontal :size="16" />
        </button>
      </ShortcutHint>
    </div>

    <DropdownMenu
      v-model:visible="isDropdownOpen"
      :items="translatedMenuItems"
      :x="dropdownX"
      :y="dropdownY"
      @select="handleMenuSelect"
    />

    <DropdownMenu
      v-model:visible="savedConnectionsOpen"
      :items="translatedSavedConnections"
      :x="savedConnectionsX"
      :y="savedConnectionsY"
      @select="handleSavedSessionSelect"
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
  transition: all var(--transition-base);
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
  transition: transform var(--transition-base);
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
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.dropdown-btn.is-active svg {
  transition: transform var(--transition-base);
  transform: rotate(180deg);
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
    background-color: var(--color-interactive-hover);
  }
}
</style>
