/**
 * Menu item constants definition.
 *
 * The `shortcut` values below are stored in their macOS form ("Cmd+...") and
 * are intentionally *not* localized here — consumers render them through
 * `formatShortcut()` from `@/core/utils/platform/platform-detection`, which
 * rewrites "Cmd+" to "Ctrl+" on non-macOS platforms for display.
 */

export const NEW_TAB_MENU_ITEMS = [
  { key: 'local', label: 'settings.newLocalTab', shortcut: 'Cmd+Shift+T' },
  { key: 'ssh', label: 'ssh.title', shortcut: 'Cmd+T' },
];
