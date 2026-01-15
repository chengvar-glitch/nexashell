/**
 * Window operation utility functions
 * Encapsulates operations related to Tauri window controls
 */

import { safeInvoke } from '@/core/utils/error-handler';

/**
 * Quit application
 */
export async function quitApp(): Promise<void> {
  await safeInvoke('quit_app');
}

/**
 * Toggle window maximization state
 */
export async function toggleMaximize(): Promise<void> {
  await safeInvoke('toggle_maximize');
}

/**
 * Minimize window
 */
export async function minimizeWindow(): Promise<void> {
  await safeInvoke('minimize_window');
}

/**
 * Close window
 */
export async function closeWindow(): Promise<void> {
  await safeInvoke('close_window');
}
