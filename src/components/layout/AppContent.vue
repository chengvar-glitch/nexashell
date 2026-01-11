<script setup lang="ts">
import { computed, inject } from 'vue';
import NexaShellHome from '@/components/home/NexaShellHome.vue';
import TerminalView from '@/components/terminal/TerminalView.vue';
import { TAB_MANAGEMENT_KEY } from '@/types';

// 注入标签管理功能
const tabManagement = inject(TAB_MANAGEMENT_KEY);
const activeTabId = tabManagement?.activeTabId;

// 根据标签类型决定显示哪个组件
const currentComponent = computed(() => {
  if (!activeTabId?.value) {
    return NexaShellHome;
  }

  const tabs = tabManagement?.tabs?.value || [];
  const activeTab = tabs.find((tab: any) => tab.id === activeTabId.value);
  
  if (!activeTab) {
    return NexaShellHome;
  }
  
  // 根据标签type字段决定显示哪个组件（解耦）
  switch (activeTab.type) {
    case 'terminal':
    case 'ssh':
      return TerminalView;
    case 'home':
    default:
      return NexaShellHome;
  }
});
</script>

<template>
  <div class="app-content">
    <component :is="currentComponent" />
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