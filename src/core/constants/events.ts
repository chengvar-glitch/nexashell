/**
 * Application event constants definition
 */

export const APP_EVENTS = {
  OPEN_SETTINGS: 'app:open-settings',
  CLOSE_DIALOG: 'app:close-dialog',
  OPEN_SSH_FORM: 'app:open-ssh-form',
  EDIT_SESSION: 'app:edit-session',
  NEW_LOCAL_TAB: 'app:new-local-tab',
  NEW_SSH_TAB: 'app:new-ssh-tab',
  CLOSE_TAB: 'app:close-tab',
  GROUPS_UPDATED: 'app:groups-updated',
  SESSION_SAVED: 'app:session-saved',
  CONNECT_SESSION: 'app:connect-session',
  SPLIT_HORIZONTAL: 'app:split-horizontal',
  SPLIT_VERTICAL: 'app:split-vertical',
  COMMAND_PALETTE: 'app:command-palette',
} as const;

export type AppEventType = (typeof APP_EVENTS)[keyof typeof APP_EVENTS];
