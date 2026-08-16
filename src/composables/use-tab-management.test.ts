import { describe, it, expect, beforeEach } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useTabManagement } from './use-tab-management';
import { useSessionStore } from '@/features/session';
import { TAB_TYPE, MAX_PANES_PER_TAB } from '@/features/tabs';
import type { SplitNode } from '@/features/tabs';

describe('useTabManagement split pane limit', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  const addTerminalTab = (tm: ReturnType<typeof useTabManagement>) => {
    tm.addTab({
      id: 'tab-1',
      label: 'Terminal 1',
      type: TAB_TYPE.TERMINAL,
      closable: true,
      panes: [{ id: 'pane-1', type: 'terminal' }],
    });
  };

  const paneCount = (tm: ReturnType<typeof useTabManagement>) => {
    const tab = tm.tabs.value.find(t => t.id === 'tab-1');
    return tab?.panes?.length ?? 0;
  };

  /** Collect every pane id that appears in the split tree. */
  const collectPaneIds = (node: SplitNode | undefined): string[] => {
    if (!node) return [];
    if (node.kind === 'pane') return [node.paneId];
    return node.children.flatMap(collectPaneIds);
  };

  it('blocks splitting once the tab reaches MAX_PANES_PER_TAB panes', () => {
    const tm = useTabManagement();
    addTerminalTab(tm);

    expect(tm.splitActivePane('vertical')).toBe('ok');
    expect(tm.splitActivePane('horizontal')).toBe('ok');
    expect(paneCount(tm)).toBe(MAX_PANES_PER_TAB);

    // Further splits must be refused and leave the tab unchanged
    expect(tm.splitActivePane('vertical')).toBe('limit');
    expect(tm.splitActivePane('horizontal')).toBe('limit');
    expect(paneCount(tm)).toBe(MAX_PANES_PER_TAB);
  });

  it('keeps split-tree pane ids in sync with tab.panes (credential cache keys)', () => {
    const tm = useTabManagement();
    addTerminalTab(tm);

    // First split goes through the `!splitTree` branch, second split through
    // replacePaneNode — both must produce ids that match tab.panes.
    expect(tm.splitActivePane('vertical')).toBe('ok');
    expect(tm.splitActivePane('horizontal')).toBe('ok');

    const tab = tm.tabs.value.find(t => t.id === 'tab-1');
    const treeIds = collectPaneIds(tab?.splitTree);
    const panesIds = (tab?.panes ?? []).map(p => p.id);
    expect(treeIds.sort()).toEqual([...panesIds].sort());
    expect(treeIds).toHaveLength(MAX_PANES_PER_TAB);
  });

  it('caches credentials under the same id the split tree uses (SSH panes)', () => {
    const tm = useTabManagement();
    const sessionStore = useSessionStore();
    // Pre-seed the source pane exactly like the SSH connect flow does:
    // a session record (with connectionParams) + a credential cache entry.
    sessionStore.cacheCredentials('pane-1', {
      password: 'secret',
      privateKeyPath: null,
      keyPassphrase: null,
    });
    sessionStore.sessions['pane-1'] = {
      id: 'pane-1',
      tabId: 'pane-1',
      type: 'ssh',
      status: 'connected',
      createdAt: new Date(),
      connectionParams: {
        serverName: 'S',
        ip: '1.2.3.4',
        port: 22,
        username: 'root',
      },
    };
    tm.addTab({
      id: 'tab-1',
      label: 'SSH 1',
      type: TAB_TYPE.SSH,
      closable: true,
      panes: [
        {
          id: 'pane-1',
          type: 'ssh',
          connect: { ip: '1.2.3.4', port: 22, username: 'root' },
        },
      ],
    });
    tm.setActivePane('pane-1');

    // Simulate what the freshly mounted pane's connect flow does: it creates
    // a session record under the pane id (credentials are already cached by
    // splitActivePane). Without this, the NEXT split can't find the source
    // pane's connectionParams.
    const seedLastPaneSession = () => {
      const tab = tm.tabs.value.find(t => t.id === 'tab-1');
      const lastPane = tab?.panes?.[tab.panes.length - 1];
      if (!lastPane) return;
      sessionStore.sessions[lastPane.id] = {
        id: lastPane.id,
        tabId: lastPane.id,
        type: 'ssh',
        status: 'connected',
        createdAt: new Date(),
        connectionParams: {
          serverName: 'S',
          ip: '1.2.3.4',
          port: 22,
          username: 'root',
        },
      };
    };

    expect(tm.splitActivePane('vertical')).toBe('ok');
    seedLastPaneSession();
    expect(tm.splitActivePane('horizontal')).toBe('ok');
    seedLastPaneSession();

    const tab = tm.tabs.value.find(t => t.id === 'tab-1');
    const treeIds = collectPaneIds(tab?.splitTree);
    // Every pane in the tree (except the pre-seeded source) must have
    // credentials cached under its own id — otherwise the pane connects with
    // an empty password and gets no input/output.
    for (const paneId of treeIds) {
      if (paneId === 'pane-1') continue;
      expect(sessionStore.getCachedCredentials(paneId)).toBeDefined();
      expect(sessionStore.getCachedCredentials(paneId)?.password).toBe(
        'secret'
      );
    }
  });

  it('reports unavailable when there is no splittable active tab', () => {
    const tm = useTabManagement();
    // Default tab is the home tab — nothing to split
    expect(tm.splitActivePane('vertical')).toBe('unavailable');
    expect(tm.splitActivePane('horizontal')).toBe('unavailable');
  });

  it('allows splitting again after a pane is closed', async () => {
    const tm = useTabManagement();
    addTerminalTab(tm);
    tm.splitActivePane('vertical');
    tm.splitActivePane('horizontal');
    expect(paneCount(tm)).toBe(MAX_PANES_PER_TAB);

    // Closing a pane frees a slot; disconnect is best-effort (no backend in
    // tests, so it may reject — closePane swallows that).
    await tm.closePane('tab-1', 'pane-1');
    expect(paneCount(tm)).toBe(MAX_PANES_PER_TAB - 1);

    expect(tm.splitActivePane('vertical')).toBe('ok');
    expect(paneCount(tm)).toBe(MAX_PANES_PER_TAB);
  });
});

