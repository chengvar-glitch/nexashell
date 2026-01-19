<script setup lang="ts">
import { computed, inject } from 'vue';
import NexaShellHome from '@/components/home/NexaShellHome.vue';
import RemoteConnectionView from '@/components/terminal/RemoteConnectionView.vue';
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
      return RemoteConnectionView;
    case 'home':
    default:
      return NexaShellHome;
  }
});

// Handle createTab event from child components
const handleCreateTab = (tab: any) => {
  emit('createTab', tab);
};

// Handle connect event from child components
const handleConnect = (data: any) => {
  emit('connect', data);
};
</script>

<template>
  <div class="app-content">
    <KeepAlive :max="10">
      <component
        :is="currentComponent"
        :key="activeTabId"
        :session-id="activeTabId"
        @create-tab="handleCreateTab"
        @connect="handleConnect"
      />
    </KeepAlive>
  </div>
</template>

<style scoped>
.app-content {
  flex: 1;
  overflow: hidden;
  background-color: var(--color-bg-secondary);
  position: relative;
  border-radius: 0 0 var(--radius-2xl) var(--radius-2xl);
  overflow: hidden;
  border: none;
}
</style>
