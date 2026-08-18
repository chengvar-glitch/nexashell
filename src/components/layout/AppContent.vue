<script setup lang="ts">
import { computed, inject } from 'vue';
import NexaShellHome from '@/components/home/NexaShellHome.vue';
import PaneContainer from '@/components/layout/PaneContainer.vue';
import { TAB_MANAGEMENT_KEY } from '@/core/types';

// Emit createTab event to parent
const emit = defineEmits(['createTab', 'connect']);

// Inject tab management functionality
const tabManagement = inject(TAB_MANAGEMENT_KEY);
if (!tabManagement) {
  throw new Error(
    'TAB_MANAGEMENT_KEY must be provided by parent component. ' +
      'Ensure App.vue provides this key via provide(TAB_MANAGEMENT_KEY, tabManagement)'
  );
}

const tabs = tabManagement.tabs;
const activeTabId = tabManagement.activeTabId;

// Determine which component to display based on tab type
const currentComponent = computed(() => {
  if (!activeTabId.value) {
    return NexaShellHome;
  }

  const activeTab = tabs.value.find(tab => tab.id === activeTabId.value);

  if (!activeTab) {
    return NexaShellHome;
  }

  // Determine which component to display based on tab type (type-driven)
  switch (activeTab.type) {
    case 'terminal':
    case 'ssh':
      return PaneContainer;
    case 'home':
    default:
      return NexaShellHome;
  }
});

// KeepAlive matches its `include` list against the cached component's NAME.
// Because KeepAlive has no per-key eviction API, we prune by component name:
// when no open tab renders a given type, that name leaves the list and ALL of
// its cached (closed-tab) vnodes are unmounted — freeing the deactivated
// RemoteConnectionView/xterm buffers instead of letting them linger until the
// max-size cache evicts them. This is the safest eviction we can drive from
// here (a global generation key would remount every open tab's xterm on each
// close). Closing one of several same-type tabs still relies on the max cap.
const openComponentNames = computed(() => {
  const names = new Set<string>();
  for (const tab of tabs.value) {
    if (tab.type === 'terminal' || tab.type === 'ssh') {
      names.add('PaneContainer');
    } else {
      names.add('NexaShellHome');
    }
  }
  return Array.from(names);
});

const currentActiveTab = computed(() => {
  if (!activeTabId.value) return null;
  return tabs.value.find(tab => tab.id === activeTabId.value) || null;
});

// Handle createTab event from child components
const handleCreateTab = (tab: import('@/features/tabs/types').Tab) => {
  emit('createTab', tab);
};

// Handle connect event from child components (SSH form data payload)
const handleConnect = (data: Record<string, unknown>) => {
  emit('connect', data);
};
</script>

<template>
  <div class="app-content">
    <Transition name="tab-switch" mode="out-in">
      <KeepAlive :max="16" :include="openComponentNames">
        <component
          :is="currentComponent"
          :key="activeTabId"
          :tab="currentActiveTab"
          @create-tab="handleCreateTab"
          @connect="handleConnect"
        />
      </KeepAlive>
    </Transition>
  </div>
</template>

<style scoped>
.app-content {
  flex: 1;
  overflow: hidden;
  background-color: var(--color-bg-secondary);
  position: relative;
  border: none;
}

/* Tab switch transition */
.tab-switch-enter-active {
  transition: opacity 0.18s var(--ease-snappy), transform 0.18s var(--ease-snappy);
}
.tab-switch-leave-active {
  transition: opacity 0.12s var(--ease-snappy), transform 0.12s var(--ease-snappy);
}
.tab-switch-enter-from {
  opacity: 0;
  transform: translateY(4px);
}
.tab-switch-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}
</style>
