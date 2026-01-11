/**
 * 菜单项常量定义
 */

export const NEW_TAB_MENU_ITEMS = [
  { key: 'local', label: 'Local Terminal', shortcut: 'Cmd+Shift+T' },
  { key: 'ssh', label: 'Remote Connection', shortcut: 'Cmd+T' },
];

export type MenuItemKey = typeof NEW_TAB_MENU_ITEMS[number]['key'];
