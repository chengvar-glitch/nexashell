<script setup lang="ts">
import {
  ref,
  onMounted,
  onBeforeUnmount,
  nextTick,
  inject,
  computed,
  watch,
} from 'vue';
import { useI18n } from 'vue-i18n';
import TabItem from '@/components/common/TabItem.vue';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import ShortcutHint from '@/components/common/ShortcutHint.vue';
import {
  Plus,
  ChevronDown,
  Terminal,
  Server,
  Home,
} from 'lucide-vue-next';
import { computeScrollTarget } from './tab-scroll';
import { type TabType } from '@/features/tabs';
import { TAB_MANAGEMENT_KEY, OPEN_SSH_FORM_KEY } from '@/core/types';
import { NEW_TAB_MENU_ITEMS } from '@/core/constants';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';
import { sortSessions } from '@/core/utils/time-utils';
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

// --- Overflow tab menu ---
// Shown when the tab strip overflows: a fixed ▾ button (outside the scroll
// container) lists every tab so any of them stays reachable by a jump.
const isOverflowing = ref(false);
const overflowMenuOpen = ref(false);
const overflowMenuX = ref(0);
const overflowMenuY = ref(0);

const updateOverflow = () => {
  const container = tabsContainerRef.value;
  if (!container) return;
  isOverflowing.value = container.scrollWidth > container.clientWidth;
};

const onToggleOverflowMenu = (event: MouseEvent) => {
  event.stopPropagation();
  const target = event.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  if (!overflowMenuOpen.value) {
    // Anchor at the button's left; DropdownMenu clamps within the viewport.
    overflowMenuX.value = rect.left;
    overflowMenuY.value = rect.bottom + 2;
  }
  overflowMenuOpen.value = !overflowMenuOpen.value;
};

const overflowTabIcon = (type: TabType) => {
  if (type === 'ssh') return Server;
  if (type === 'terminal') return Terminal;
  return Home;
};

const overflowTabsItems = computed(() =>
  tabs.value.map(tab => ({
    key: tab.id,
    label: tab.label,
    icon: overflowTabIcon(tab.type),
    active: tab.id === activeTabId.value,
  }))
);

const handleOverflowTabSelect = async (key: string) => {
  overflowMenuOpen.value = false;
  if (key === activeTabId.value) return;
  tabManagement.setActiveTab(key);
  await nextTick();
  scrollToActiveTab();
};

// Re-evaluate overflow once the tab strip has been updated in the DOM.
watch(
  () => tabs.value.length,
  () => nextTick(updateOverflow)
);

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

const handleCloseOthers = async (id: string) => {
  await tabManagement.closeOthers(id);
};

const handleCloseAll = async () => {
  await tabManagement.closeAllTabs();
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
  return sortSessions(list);
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

// Scroll to the currently active tab using viewport-relative rects
// (scroll-aware, unlike offsetLeft which ignores ancestor scrolling).
const scrollToActiveTab = () => {
  const container = tabsContainerRef.value;
  if (!container) return;

  const activeTabElement = container.querySelector(
    `.tab-item[data-id="${activeTabId.value}"]`
  ) as HTMLElement | null;
  if (!activeTabElement) return;

  const containerRect = container.getBoundingClientRect();
  if (containerRect.width === 0) return;

  const tabRect = activeTabElement.getBoundingClientRect();
  const target = computeScrollTarget(
    { scrollLeft: container.scrollLeft, clientWidth: container.clientWidth },
    { left: tabRect.left, right: tabRect.right },
    containerRect.left
  );

  if (target !== null) {
    container.scrollTo({ left: target, behavior: 'smooth' });
  }
};

const handleWindowResize = () => {
  scrollToActiveTab();
  updateOverflow();
};

onMounted(() => {
  eventBus.on(APP_EVENTS.CLOSE_TAB, handleCloseTabShortcut);
  eventBus.on(APP_EVENTS.NEW_LOCAL_TAB, handleNewLocalTab);
  eventBus.on(APP_EVENTS.NEW_SSH_TAB, handleNewSSHTab);
  eventBus.on(APP_EVENTS.SESSION_SAVED, invalidateSavedConnectionsCache);

  window.addEventListener('resize', handleWindowResize);

  // Measure overflow after the first render (and after tab count changes,
  // once the new tab is actually in the DOM).
  nextTick(updateOverflow);
});

onBeforeUnmount(() => {
  eventBus.off(APP_EVENTS.CLOSE_TAB, handleCloseTabShortcut);
  eventBus.off(APP_EVENTS.NEW_LOCAL_TAB, handleNewLocalTab);
  eventBus.off(APP_EVENTS.NEW_SSH_TAB, handleNewSSHTab);
  eventBus.off(APP_EVENTS.SESSION_SAVED, invalidateSavedConnectionsCache);

  window.removeEventListener('resize', handleWindowResize);
});
</script>

<template>
  <div class="app-tabs" data-tauri-drag-region>
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
          @close-others="handleCloseOthers"
          @close-all="handleCloseAll"
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

    <ShortcutHint
      v-if="isOverflowing"
      :text="t('tabs.overflowTabs')"
      position="bottom"
    >
      <button
        class="action-btn overflow-tabs-btn"
        :class="{ 'is-active': overflowMenuOpen }"
        :aria-label="t('tabs.overflowTabs')"
        @click="onToggleOverflowMenu"
      >
        <ChevronDown :size="14" />
      </button>
    </ShortcutHint>

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

    <DropdownMenu
      v-model:visible="overflowMenuOpen"
      :items="overflowTabsItems"
      :x="overflowMenuX"
      :y="overflowMenuY"
      max-height="60vh"
      @select="handleOverflowTabSelect"
    />
  </div>
</template>

<style scoped>
.app-tabs {
  display: flex;
  align-items: center;
  background-color: transparent;
  padding: 0 4px;
  height: 100%;
  min-width: 0;
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

.overflow-tabs-btn {
  margin-left: 4px;
  flex-shrink: 0;
}

.overflow-tabs-btn.is-active {
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
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
