/**
 * Tab management Composable
 * Provides tab state management and operation methods
 *
 * Integrated with Pinia session store for proper session cleanup
 */

import { ref } from 'vue';
import { v4 as uuidv4 } from 'uuid';
import { type Tab, type Pane, type SplitNode, type SplitDirection, DEFAULT_TAB } from '@/features/tabs';
import { useSessionStore } from '@/features/session';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('TAB_MANAGEMENT');

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
  const activePaneId = ref<string>('');

  const setActiveTab = (id: string) => {
    activeTabId.value = id as string;
    const tab = tabs.value.find(t => t.id === id);
    if (tab && tab.panes && tab.panes.length > 0) {
      activePaneId.value = tab.panes[0].id;
    }
  };

  const setActivePane = (paneId: string) => {
    activePaneId.value = paneId;
  };

  const addTab = (tab: Tab) => {
    tabs.value.push(tab);
    activeTabId.value = tab.id as string;
    if (tab.panes && tab.panes.length > 0) {
      activePaneId.value = tab.panes[0].id;
    }
  };

  const replacePaneNode = (node: SplitNode, paneId: string, direction: SplitDirection): SplitNode => {
    if (node.kind === 'pane' && node.paneId === paneId) {
      const newPaneId = uuidv4();
      return {
        kind: 'split',
        direction,
        children: [
          { kind: 'pane', paneId: node.paneId },
          { kind: 'pane', paneId: newPaneId },
        ],
        sizes: [50, 50],
      };
    }
    if (node.kind === 'split') {
      return {
        ...node,
        children: node.children.map(child => replacePaneNode(child, paneId, direction)),
      };
    }
    return node;
  };

  const removePaneNode = (node: SplitNode, paneId: string): SplitNode | null => {
    if (node.kind === 'pane') {
      return node.paneId === paneId ? null : node;
    }
    // Keep (child, size) pairs in lockstep so sizes stay aligned with
    // children after a subtree collapses — filtering sizes against the
    // ORIGINAL children array desyncs the two and corrupts the tree.
    const kept: Array<{ child: SplitNode; size: number }> = [];
    node.children.forEach((child, i) => {
      const result = removePaneNode(child, paneId);
      if (result !== null) {
        kept.push({ child: result, size: node.sizes[i] ?? 100 / node.children.length });
      }
    });
    if (kept.length === 0) return null;
    if (kept.length === 1) return kept[0].child;
    const total = kept.reduce((sum, k) => sum + k.size, 0);
    const sizes =
      total > 0
        ? kept.map(k => (k.size / total) * 100)
        : kept.map(() => 100 / kept.length);
    return {
      kind: 'split',
      direction: node.direction,
      children: kept.map(k => k.child),
      sizes,
    };
  };

  const splitActivePane = async (direction: SplitDirection) => {
    const tab = tabs.value.find(t => t.id === activeTabId.value);
    if (!tab || !tab.panes || tab.type === 'home') return;
    const sourcePaneId = activePaneId.value;
    if (!sourcePaneId) return;

    const sessionStore = useSessionStore();
    const newPaneId = uuidv4();
    const newPane: Pane = { id: newPaneId, type: tab.type === 'ssh' ? 'ssh' : 'terminal' };

    // Resolve credentials BEFORE mutating any state — if they're missing we
    // bail out cleanly instead of leaving a dangling pane behind.
    const creds = sessionStore.getCachedCredentials(sourcePaneId);
    if (tab.type === 'ssh' && !creds) {
      logger.warn('No cached credentials for splitting SSH pane', { paneId: sourcePaneId });
      return;
    }

    const prevSplitTree = tab.splitTree;
    const prevPaneCount = tab.panes.length;
    const prevActivePane = activePaneId.value;

    if (!tab.splitTree) {
      tab.splitTree = {
        kind: 'split',
        direction,
        children: [
          { kind: 'pane', paneId: tab.panes[0].id },
          { kind: 'pane', paneId: newPaneId },
        ],
        sizes: [50, 50],
      };
    } else {
      tab.splitTree = replacePaneNode(tab.splitTree, sourcePaneId, direction);
    }
    tab.panes.push(newPane);

    try {
      if (tab.type === 'ssh') {
        const cp = sessionStore.getSession(sourcePaneId)?.connectionParams;
        if (!cp) throw new Error('Source session connection params missing');
        await sessionStore.createSSHSession(
          newPaneId,
          newPaneId,
          cp.serverName,
          cp.ip,
          cp.port,
          cp.username,
          creds!.password || '',
          creds!.privateKeyPath || null,
          creds!.keyPassphrase || null,
          80,
          24
        );
      } else {
        await sessionStore.createLocalSession(newPaneId, newPaneId, 80, 24);
      }
    } catch (error) {
      // Roll back the tree mutation so a failed split leaves no orphan pane
      logger.error('Failed to create split pane session, rolling back', error);
      tab.panes.length = prevPaneCount;
      tab.splitTree = prevSplitTree;
      activePaneId.value = prevActivePane;
      return;
    }

    activePaneId.value = newPaneId;
  };

  const closePane = async (tabId: string, paneId: string): Promise<void> => {
    const tab = tabs.value.find(t => t.id === tabId);
    if (!tab || !tab.panes) return;

    const sessionStore = useSessionStore();
    try {
      await sessionStore.disconnectSession(paneId);
    } catch (error) {
      logger.error('Error disconnecting pane session', { paneId, error });
    }

    const paneIndex = tab.panes.findIndex(p => p.id === paneId);
    if (paneIndex !== -1) {
      tab.panes.splice(paneIndex, 1);
    }

    if (tab.panes.length === 0) {
      await closeTab(tabId);
      return;
    }

    if (tab.splitTree) {
      const newTree = removePaneNode(tab.splitTree, paneId);
      // Re-normalize sizes to 100% — removing a child leaves the sum < 100,
      // which would render leftover empty space at the container's end
      if (newTree && newTree.kind === 'split') {
        const total = newTree.sizes.reduce((a, b) => a + b, 0);
        if (total > 0) {
          newTree.sizes = newTree.sizes.map(s => (s / total) * 100);
        }
      }
      tab.splitTree = newTree || undefined;
    }

    if (activePaneId.value === paneId) {
      activePaneId.value = tab.panes[0]?.id || '';
    }
  };

  /**
   * Close tab with proper async cleanup handling
   *
   * Strategy:
   * 1. If closing active terminal tab, switch to another tab first
   * 2. Clean up associated sessions using Pinia store
   * 3. Finally remove from tabs list
   */
  // Re-entrancy guard: rapid repeated closes (e.g. double Cmd+W) would
  // otherwise run the async cleanup twice and leave a stale active-tab pointer
  const closingTabIds = new Set<string>();
  const closeTab = (id: string): Promise<void> => {
    if (closingTabIds.has(id)) return Promise.resolve();
    closingTabIds.add(id);
    return new Promise(resolve => {
      const finish = () => {
        closingTabIds.delete(id);
        resolve();
      };
      const index = tabs.value.findIndex(tab => tab.id === id);
      if (index === -1) {
        finish();
        return;
      }

      const tabToClose = tabs.value[index];
      const isCurrentActiveTab = id === activeTabId.value;
      const isTerminalTab =
        tabToClose &&
        (tabToClose.type === 'terminal' || tabToClose.type === 'ssh');

      if (isCurrentActiveTab && tabs.value.length > 1) {
        const newIndex = Math.min(index, tabs.value.length - 2);
        activeTabId.value = tabs.value[newIndex].id as string;
      }

      if (isTerminalTab) {
        const sessionStore = useSessionStore();
        const paneIds = (tabToClose.panes || []).map(p => p.id);
        logger.debug(`Starting cleanup for tab: ${id}, panes: ${paneIds.join(',')}`);
        sessionStore
          .disconnectSessions(paneIds)
          .catch(error => {
            logger.error(`Error disconnecting sessions for tab: ${id}`, error);
          })
          .finally(() => {
            const tabIndex = tabs.value.findIndex(tab => tab.id === id);
            if (tabIndex !== -1) {
              tabs.value.splice(tabIndex, 1);
            }
            finish();
          });
      } else {
        tabs.value.splice(index, 1);
        finish();
      }
    });
  };

  const getActiveTab = () => {
    return tabs.value.find(tab => tab.id === activeTabId.value);
  };

  return {
    tabs,
    activeTabId,
    activePaneId,
    setActiveTab,
    setActivePane,
    addTab,
    closeTab,
    splitActivePane,
    closePane,
    getActiveTab,
  };
}