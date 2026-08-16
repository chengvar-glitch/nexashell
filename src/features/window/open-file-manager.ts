/**
 * Opens a standalone file-manager window for an SSH session.
 *
 * The window embeds the session id in its label (`file-manager-{sessionId}`)
 * so its entry component can recover it. Calling this again for a session that
 * already has such a window open focuses the existing one instead of spawning
 * a duplicate.
 */
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('FILE_MANAGER_WINDOW');

export const fileManagerWindowLabel = (sessionId: string): string =>
  `file-manager-${sessionId}`;

/**
 * Open (or focus) the file-manager window for an SSH session.
 *
 * Returns true when a window is available (either created or already shown);
 * false when it could not be created.
 */
export async function openFileManagerWindow(sessionId: string): Promise<boolean> {
  if (!sessionId) return false;

  const label = fileManagerWindowLabel(sessionId);

  // Reuse an existing window for this session if one is still open.
  const existing = await WebviewWindow.getByLabel(label).catch(err => {
    logger.error('Failed to look up file-manager window', err);
    return null;
  });
  if (existing) {
    try {
      await existing.show();
      await existing.setFocus();
    } catch (err) {
      logger.error('Failed to focus existing file-manager window', err);
    }
    return true;
  }

  try {
    const win = new WebviewWindow(label, {
      url: 'filemanager.html',
      title: 'NexaShell File Manager',
      width: 920,
      height: 720,
      center: true,
      resizable: true,
    });
    await win.once('tauri://created', () => {
      logger.info('File-manager window created', { label });
    });
    await win.once('tauri://error', e => {
      logger.error('File-manager window failed to load', e);
    });
    return true;
  } catch (err) {
    logger.error('Failed to open file-manager window', err);
    return false;
  }
}
