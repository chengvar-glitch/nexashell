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
