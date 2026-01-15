/**
 * Tab management Composable
 * Provides tab state management and operation methods
 *
 * Integrated with Pinia session store for proper session cleanup
 */

import { ref } from 'vue';
import { type Tab, DEFAULT_TAB } from '@/features/tabs';
import { useSessionStore } from '@/features/session';

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

  /**
   * Close tab with proper async cleanup handling
   *
   * Strategy:
   * 1. If closing active terminal tab, switch to another tab first
   * 2. Clean up associated session using Pinia store
   * 3. Finally remove from tabs list
   */
  const closeTab = (id: string): Promise<void> => {
    return new Promise(resolve => {
      const index = tabs.value.findIndex(tab => tab.id === id);
      if (index === -1) {
        resolve();
        return;
      }

      const tabToClose = tabs.value[index];
      const isCurrentActiveTab = id === activeTabId.value;
      const isTerminalTab =
        tabToClose &&
        (tabToClose.type === 'terminal' || tabToClose.type === 'ssh');

      // Step 1: Handle active tab switching first
      if (isCurrentActiveTab && tabs.value.length > 1) {
        const newIndex = Math.min(index, tabs.value.length - 2);
        activeTabId.value = tabs.value[newIndex].id as string;
      }

      // Step 2: Clean up session if it's a terminal tab
      if (isTerminalTab) {
        // Use Pinia store to cleanup session
        const sessionStore = useSessionStore();
        console.log('[TAB_MANAGEMENT] Starting cleanup for tab:', id);
        sessionStore
          .disconnectByTabId(id)
          .catch(error => {
            console.error(
              '[TAB_MANAGEMENT] Error disconnecting session for tab:',
              id,
              error
            );
          })
          .finally(() => {
            // Step 3: Remove tab after async cleanup is done
            const tabIndex = tabs.value.findIndex(tab => tab.id === id);
            if (tabIndex !== -1) {
              tabs.value.splice(tabIndex, 1);
              console.log('[TAB_MANAGEMENT] Tab removed after cleanup:', id);
              // Additional log to confirm mapping removed
              console.log('[TAB_MANAGEMENT] Finished cleanup for tab:', id);
            }
            resolve();
          });
      } else {
        // For non-terminal tabs, remove immediately
        tabs.value.splice(index, 1);
        resolve();
      }
    });
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
