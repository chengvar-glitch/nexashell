/**
 * Tab type definitions
 */

export type TabType = 'home' | 'terminal' | 'ssh';

export type SplitDirection = 'horizontal' | 'vertical';

/**
 * Connection descriptor for a split pane — lets the freshly-mounted
 * RemoteConnectionView run its own full connect flow (welcome banner
 * replay, error handling) instead of attaching to an already-established
 * session and missing the initial output. Only NON-sensitive fields live
 * here (it's part of reactive tab state); credentials are resolved from the
 * session store's non-reactive cache at connect time.
 */
export interface PaneConnect {
  ip: string;
  port: number;
  username: string;
}

export type SplitNode =
  | { kind: 'pane'; paneId: string; connect?: PaneConnect }
  | { kind: 'split'; direction: SplitDirection; children: SplitNode[]; sizes: number[] };

/** Outcome of a split request: performed, blocked by the pane cap, or
 *  unavailable for other reasons (e.g. home tab, missing credentials). */
export type SplitPaneResult = 'ok' | 'limit' | 'unavailable';

export interface Pane {
  id: string;
  type: 'terminal' | 'ssh';
  connect?: PaneConnect;
}

export interface Tab {
  id: string;
  label: string;
  type: TabType;
  closable: boolean;
  panes?: Pane[];
  splitTree?: SplitNode;
}

export interface TabManagement {
  tabs: import('vue').Ref<Tab[]>;
  activeTabId: import('vue').Ref<string>;
  activePaneId: import('vue').Ref<string>;
  setActiveTab: (id: string) => void;
  setActivePane: (id: string) => void;
  addTab: (tab: Tab) => void;
  closeTab: (id: string) => Promise<void>;
  /** Splits the active pane. Returns 'ok' when a pane was created, 'limit'
   *  when the per-tab pane cap blocks it, or 'unavailable' otherwise. */
  splitActivePane: (direction: SplitDirection) => SplitPaneResult;
  closePane: (tabId: string, paneId: string) => Promise<void>;
  getActiveTab: () => Tab | undefined;
  /** Id of the active SSH pane (= its runtime session id), or null. */
  getActiveSshPaneId: () => string | null;
}

// Export constants for tab types
export const TAB_TYPE = {
  HOME: 'home' as const,
  TERMINAL: 'terminal' as const,
  SSH: 'ssh' as const,
};
