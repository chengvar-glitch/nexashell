/**
 * Tab management Composable
 * Provides tab state management and operation methods
 *
 * Integrated with Pinia session store for proper session cleanup
 */

import { ref } from 'vue';
import { type Tab, type Pane, type PaneConnect, type SplitNode, type SplitDirection, type SplitPaneResult, DEFAULT_TAB, MAX_PANES_PER_TAB } from '@/features/tabs';
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

  /**
   * Replace the pane `paneId` with a split node whose second child is the
   * NEW pane. `newPaneId` MUST be the same id splitActivePane uses for
   * `tab.panes` and the credential cache — generating a fresh id here would
   * render a pane whose session id has no cached credentials, so the second
   * split's new pane would try to authenticate with an empty password and
   * end up with no input/output.
   */
  const replacePaneNode = (
    node: SplitNode,
    paneId: string,
    direction: SplitDirection,
    newPaneId: string,
    newConnect?: PaneConnect
  ): SplitNode => {
    if (node.kind === 'pane' && node.paneId === paneId) {
      return {
        kind: 'split',
        direction,
        children: [
          { kind: 'pane', paneId: node.paneId },
          { kind: 'pane', paneId: newPaneId, connect: newConnect },
        ],
        sizes: [50, 50],
      };
    }
    if (node.kind === 'split') {
      return {
        ...node,
        children: node.children.map(child =>
          replacePaneNode(child, paneId, direction, newPaneId, newConnect)
        ),
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

  const splitActivePane = (direction: SplitDirection): SplitPaneResult => {
    const tab = tabs.value.find(t => t.id === activeTabId.value);
    if (!tab || !tab.panes || tab.panes.length === 0 || tab.type === 'home') {
      return 'unavailable';
    }
    const sourcePaneId = activePaneId.value;
    if (!sourcePaneId) return 'unavailable';

    // Per-tab pane cap: refuse further splits once the tab is full.
    if (tab.panes.length >= MAX_PANES_PER_TAB) {
      logger.warn('Split blocked: pane limit reached', {
        tabId: tab.id,
        panes: tab.panes.length,
        max: MAX_PANES_PER_TAB,
      });
      return 'limit';
    }

    const sessionStore = useSessionStore();
    const newPaneId = crypto.randomUUID();

    // Build a connect descriptor for SSH splits. We deliberately do NOT
    // create the session here — the freshly mounted RemoteConnectionView
    // runs its own connect flow (welcome-banner replay, error handling),
    // matching the main connection path. Credentials are stashed in the
    // session store's NON-reactive cache (keyed by the new pane id); the
    // pane itself only carries non-sensitive fields (reactive tab state).
    let connect: PaneConnect | undefined;
    if (tab.type === 'ssh') {
      const cp = sessionStore.getSession(sourcePaneId)?.connectionParams;
      const creds = sessionStore.getCachedCredentials(sourcePaneId);
      if (!cp || !creds) {
        logger.warn('No connection info for splitting SSH pane', { paneId: sourcePaneId });
        return 'unavailable';
      }
      sessionStore.cacheCredentials(newPaneId, {
        password: creds.password,
        privateKeyPath: creds.privateKeyPath,
        keyPassphrase: creds.keyPassphrase,
      });
      connect = {
        ip: cp.ip,
        port: cp.port,
        username: cp.username,
      };
    }

    const newPane: Pane = {
      id: newPaneId,
      type: tab.type === 'ssh' ? 'ssh' : 'terminal',
      connect,
    };

    if (!tab.splitTree) {
      tab.splitTree = {
        kind: 'split',
        direction,
        children: [
          { kind: 'pane', paneId: sourcePaneId },
          { kind: 'pane', paneId: newPaneId, connect },
        ],
        sizes: [50, 50],
      };
    } else {
      tab.splitTree = replacePaneNode(
        tab.splitTree,
        sourcePaneId,
        direction,
        newPaneId,
        connect
      );
    }
    tab.panes.push(newPane);

    activePaneId.value = newPaneId;
    return 'ok';
  };

  // Re-entrancy guard: rapid repeated closes (e.g. double-click) would
  // otherwise run the async disconnect twice on the same pane.
  const closingPaneIds = new Set<string>();
  const closePane = async (tabId: string, paneId: string): Promise<void> => {
    if (closingPaneIds.has(paneId)) return;
    closingPaneIds.add(paneId);

    const tab = tabs.value.find(t => t.id === tabId);
    if (!tab || !tab.panes) {
      closingPaneIds.delete(paneId);
      return;
    }

    const sessionStore = useSessionStore();
    try {
      await sessionStore.disconnectSession(paneId);
    } catch (error) {
      logger.error('Error disconnecting pane session', { paneId, error });
      // Keep the pane and its session record in place: removing the pane
      // while the backend still holds a live connection would orphan it with
      // no way to close it. The re-entrancy guard is reset so the user can
      // retry the close.
      closingPaneIds.delete(paneId);
      return;
    }

    const paneIndex = tab.panes.findIndex(p => p.id === paneId);
    if (paneIndex !== -1) {
      tab.panes.splice(paneIndex, 1);
    }

    if (tab.panes.length === 0) {
      closingPaneIds.delete(paneId);
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

    closingPaneIds.delete(paneId);
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
        // When closing a non-final tab, adopt the one to its right; when
        // closing the final tab, adopt the one to its left. The previous
        // `Math.min(index, length - 2)` pointed at the tab being deleted for
        // any non-final index, leaving a dangling activeTabId and a blank
        // workspace. `length > 1` guarantees index-1 is never negative.
        const newIndex = index < tabs.value.length - 1 ? index + 1 : index - 1;
        const nextTab = tabs.value[newIndex];
        activeTabId.value = nextTab.id as string;
        // Re-derive the active pane so the newly active tab renders immediately
        // instead of keeping a stale pane id from the closing tab.
        activePaneId.value =
          nextTab.panes && nextTab.panes.length > 0 ? nextTab.panes[0].id : '';
      }

      if (isTerminalTab) {
        const sessionStore = useSessionStore();
        const paneIds = (tabToClose.panes || []).map(p => p.id);
        logger.debug(`Starting cleanup for tab: ${id}, panes: ${paneIds.join(',')}`);
        let disconnected = false;
        sessionStore
          .disconnectSessions(paneIds)
          .then(() => {
            disconnected = true;
          })
          .catch(error => {
            logger.error(`Error disconnecting sessions for tab: ${id}`, error);
            // Keep the tab open: removing it while the backend still holds
            // live connections would orphan them with no way to close them.
            // Retrying the close re-runs the disconnect.
          })
          .finally(() => {
            if (disconnected) {
              const tabIndex = tabs.value.findIndex(tab => tab.id === id);
              if (tabIndex !== -1) {
                tabs.value.splice(tabIndex, 1);
              }
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

  /**
   * Resolve the active SSH pane's id (which equals its runtime session id),
   * or null when the active tab is not an SSH tab or has no panes.
   */
  const getActiveSshPaneId = (): string | null => {
    const tab = getActiveTab();
    if (!tab || tab.type !== 'ssh' || !tab.panes || tab.panes.length === 0) {
      return null;
    }
    return activePaneId.value || tab.panes[0].id || null;
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
    getActiveSshPaneId,
  };
}