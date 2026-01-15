/**
 * Tab operation utility functions
 */

import { APP_EVENTS } from '@/core/constants';

/**
 * Create new local terminal tab
 */
export function createNewLocalTab(): void {
  window.dispatchEvent(new CustomEvent(APP_EVENTS.NEW_LOCAL_TAB));
}

/**
 * Focus search box
 */
export function focusSearch(): void {
  window.dispatchEvent(new CustomEvent(APP_EVENTS.FOCUS_SEARCH));
}
