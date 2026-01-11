/**
 * Tab management Composable
 * Provides tab state management and operation methods
 */

import { ref } from 'vue';
import type { Tab } from '@/types';
import { DEFAULT_TAB } from '@/constants';

export function useTabManagement() {
  const tabs = ref<Tab[]>([
    {
      id: DEFAULT_TAB.ID,
      label: DEFAULT_TAB.LABEL,
      type: DEFAULT_TAB.TYPE,
      closable: false,
    },
  ]);

  const activeTabId = ref<string>(DEFAULT_TAB.ID);

  const setActiveTab = (id: string) => {
    activeTabId.value = id as string;
  };

  const addTab = (tab: Tab) => {
    tabs.value.push(tab);
    activeTabId.value = tab.id as string;
  };

  const closeTab = (id: string) => {
    const index = tabs.value.findIndex(tab => tab.id === id);
    if (index === -1) return;

    tabs.value.splice(index, 1);

    // If closing the active tab, switch to adjacent tab
    if (id === activeTabId.value && tabs.value.length > 0) {
      const newIndex = Math.min(index, tabs.value.length - 1);
      activeTabId.value = tabs.value[newIndex].id as string;
    }
  };

  const getActiveTab = () => {
    return tabs.value.find(tab => tab.id === activeTabId.value);
  };

  return {
    tabs,
    activeTabId,
    setActiveTab,
    addTab,
    closeTab,
    getActiveTab,
  };
}
