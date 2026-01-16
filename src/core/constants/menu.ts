/**
 * Menu item constants definition
 */

export const NEW_TAB_MENU_ITEMS = [
  { key: 'local', label: 'home.newSession', shortcut: 'Cmd+Shift+T' },
  { key: 'ssh', label: 'ssh.title', shortcut: 'Cmd+T' },
];

export type MenuItemKey = (typeof NEW_TAB_MENU_ITEMS)[number]['key'];
