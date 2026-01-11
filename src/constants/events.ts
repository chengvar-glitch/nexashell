/**
 * Application event constants definition
 */

export const APP_EVENTS = {
  OPEN_SETTINGS: 'app:open-settings',
  CLOSE_DIALOG: 'app:close-dialog',
  OPEN_SSH_FORM: 'app:open-ssh-form',
  NEW_LOCAL_TAB: 'app:new-local-tab',
  NEW_SSH_TAB: 'app:new-ssh-tab',
  CLOSE_TAB: 'app:close-tab',
  NEW_TAB: 'app:new-tab',
  FOCUS_SEARCH: 'app:focus-search',
} as const;

export type AppEventType = typeof APP_EVENTS[keyof typeof APP_EVENTS];
