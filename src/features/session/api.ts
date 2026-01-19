/**
 * Session API Layer
 *
 * Lightweight wrapper around Tauri `invoke` calls for SSH-related commands.
 * Responsibilities:
 * - Encapsulate RPC argument shapes and return types
 * - Centralize error logging for Tauri IPC
 * - Provide a stable, testable interface for the Pinia store and UI
 */

import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '@/core/utils/logger';
import type { SavedSession } from './types';

const logger = createLogger('SESSION_API');

/**
 * SessionAPI class
 *
 * Encapsulates all Tauri invoke calls for session management.
 * Each method is simple and focused on a single Tauri command.
 */
class SessionAPI {
  /**
   * Request the backend to establish an SSH session.
   *
   * Optimized for Tauri 2.0+ best practices:
   * - Structured error handling (SshError enum)
   * - Type-safe session management (SessionId newtype)
   * - Clean API surface (no redundant parameters)
   *
   * @param sessionId Unique session identifier (corresponds to tab id)
   * @param ip Remote host IP or hostname
   * @param port Remote port (typically 22)
   * @param username Login username
   * @param password Password or empty string if using key-based auth
   * @param cols Initial terminal columns
   * @param rows Initial terminal rows
   * @returns Promise that resolves when the invoke completes. The actual SSH
   *          connection is established asynchronously on the Rust side.
   * @throws SshError with detailed context (ConnectionFailed, AuthenticationFailed, etc.)
   */
  async connectSSH(
    sessionId: string,
    ip: string,
    port: number,
    username: string,
    password: string,
    cols: number,
    rows: number
  ): Promise<void> {
    try {
      const params = {
        sessionId: sessionId,
        ip,
        port,
        username,
        password,
        cols,
        rows,
      };

      console.debug('[DEBUG] sessionApi.connectSSH invoke', params);

      // Tauri 2.0+ optimized: Direct invoke with camelCase parameters matching JavaScript style
      await invoke<void>('connect_ssh', params as Record<string, unknown>);

      console.debug('[DEBUG] sessionApi.connectSSH returned', { sessionId });
      logger.info('SSH connection initiated', { sessionId, ip, port });
    } catch (error) {
      logger.error('Failed to connect SSH', error);
      throw error;
    }
  }

  /**
   * Request backend to disconnect and tear down the SSH session.
   * @param sessionId Unique session identifier to disconnect
   * @returns Promise that resolves when the invoke completes
   */
  async disconnectSSH(sessionId: string): Promise<void> {
    try {
      const params = { sessionId };

      console.debug('[DEBUG] sessionApi.disconnectSSH invoke', params);

      // Tauri 2.0+ optimized: camelCase parameters
      await invoke<void>('disconnect_ssh', params as Record<string, unknown>);

      console.debug('[DEBUG] sessionApi.disconnectSSH returned', { sessionId });
      logger.info('SSH disconnection initiated', { sessionId });
    } catch (error) {
      logger.error('Failed to disconnect SSH', error);
      throw error;
    }
  }

  /**
   * Send a raw input string to the SSH session on the backend.
   * This method forwards keystrokes or pasted text to the Rust task.
   * @param sessionId Unique session identifier
   * @param input Raw byte string to send to the remote PTY
   */
  async sendSSHInput(sessionId: string, input: string): Promise<void> {
    try {
      const params = {
        sessionId,
        input,
      };

      console.debug('[DEBUG] sessionApi.sendSSHInput invoke', {
        sessionId: params.sessionId,
        inputLen: input.length,
      });

      // Tauri 2.0+ optimized: camelCase parameters
      await invoke<void>('send_ssh_input', params as Record<string, unknown>);

      console.debug('[DEBUG] sessionApi.sendSSHInput returned', { sessionId });
    } catch (error) {
      logger.error('Failed to send SSH input', error);
      throw error;
    }
  }

  /**
   * Get buffered initial SSH output (welcome banner, etc.)
   * @param sessionId Unique session identifier
   * @returns Promise resolving to array of buffered output chunks
   */
  async getBufferedSSHOutput(
    sessionId: string
  ): Promise<Array<{ seq: number; output: string; ts: number }>> {
    try {
      const params = { sessionId };
      const result = await invoke<
        Array<{ seq: number; output: string; ts: number }>
      >('get_buffered_ssh_output', params as Record<string, unknown>);
      return (
        (result as Array<{ seq: number; output: string; ts: number }>) || []
      );
    } catch (error) {
      logger.error('Failed to get buffered SSH output', error);
      return [];
    }
  }

  /**
   * Save SSH session metadata to the database.
   * This saves all session information except sensitive data (passwords, passphrases).
   * Optionally associates the session with groups and tags.
   *
   * @param addr SSH server address (host or IP)
   * @param port SSH server port
   * @param serverName Human-friendly session name
   * @param username SSH username
   * @param authType Authentication type ('password' or 'key')
   * @param privateKeyPath Path to private key file (optional)
   * @param isFavorite Whether the session is favorited (optional)
   * @param groupIds List of group IDs to associate with this session (optional)
   * @param tagIds List of tag IDs to associate with this session (optional)
   * @returns Promise resolving to the UUID of the newly created session
   * @throws Error if the save operation fails
   */
  async saveSession(
    addr: string,
    port: number,
    serverName: string,
    username: string,
    authType: string,
    privateKeyPath?: string,
    isFavorite?: boolean,
    groupIds?: string[],
    tagIds?: string[]
  ): Promise<string> {
    try {
      const params = {
        addr,
        port: Math.floor(port),
        serverName,
        username,
        authType,
        privateKeyPath: privateKeyPath || null,
        isFavorite: isFavorite ?? null,
        groupIds: groupIds || null,
        tagIds: tagIds || null,
      };

      console.debug('[DEBUG] sessionApi.saveSession invoke', {
        addr: params.addr,
        port: params.port,
        serverName: params.serverName,
        username: params.username,
        isFavorite: params.isFavorite,
        groupCount: groupIds?.length || 0,
        tagCount: tagIds?.length || 0,
      });

      // Invoke save_session Tauri command
      const sessionId = await invoke<string>(
        'save_session',
        params as Record<string, unknown>
      );

      console.debug('[DEBUG] sessionApi.saveSession returned', { sessionId });
      logger.info('SSH session saved', {
        sessionId,
        serverName,
        addr,
        port,
      });

      return sessionId;
    } catch (error) {
      logger.error('Failed to save SSH session', error);
      throw error;
    }
  }

  /**
   * Toggle the favorite status of a session in the database.
   * @param id Session UUID
   * @param isFavorite New favorite status
   */
  async toggleFavorite(id: string, isFavorite: boolean): Promise<void> {
    try {
      await invoke('toggle_favorite', { id, isFavorite });
      logger.info('Session favorite status toggled', { id, isFavorite });
    } catch (error) {
      logger.error('Failed to toggle session favorite status', error);
      throw error;
    }
  }

  /**
   * List all saved SSH sessions from the database.
   * @returns Promise resolving to an array of saved sessions
   */
  async listSessions(): Promise<SavedSession[]> {
    try {
      const sessions = await invoke<SavedSession[]>('list_sessions');
      return sessions || [];
    } catch (error) {
      logger.error('Failed to list sessions', error);
      return [];
    }
  }
}

// Singleton instance
export const sessionApi = new SessionAPI();
