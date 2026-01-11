/**
 * 标签页类型定义
 */

export type TabType = 'home' | 'terminal' | 'ssh';

export interface Tab {
  id: string;
  label: string;
  type: TabType;
  closable: boolean;
}

export interface TabManagement {
  tabs: import('vue').Ref<Tab[]>;
  activeTabId: import('vue').Ref<string>;
  setActiveTab: (id: string) => void;
  addTab: (tab: Tab) => void;
  closeTab: (id: string) => void;
}
