import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '@/core/utils/logger';
import type { ImportResult, SavedSession } from './types';

const logger = createLogger('SESSION_API');

/** Payload for `save_session_with_credentials`. */
export interface SaveSessionWithCredentialsPayload {
  id: string | null;
  addr: string;
  port: number;
  serverName: string;
  username: string;
  authType: string;
  privateKeyPath: string | null;
  password: string | null;
  keyPassphrase: string | null;
  clearCredentials: boolean;
  groupIds: string[] | null;
  tagIds: string[] | null;
}

class SessionAPI {
  async connectSSH(
    sessionId: string,
    ip: string,
    port: number,
    username: string,
    password: string,
    privateKeyPath?: string | null,
    keyPassphrase?: string | null,
    cols: number = 80,
    rows: number = 24
  ): Promise<void> {
    try {
      logger.debug('connectSSH invoke', {
        sessionId,
        ip,
        port,
        hasPassword: !!password,
        hasKeyPath: !!privateKeyPath,
      });

      await invoke<void>('connect_ssh', {
        sessionId,
        ip,
        port,
        username,
        password,
        privateKeyPath: privateKeyPath || null,
        keyPassphrase: keyPassphrase || null,
        cols,
        rows,
      } as Record<string, unknown>);

      logger.info('SSH connection initiated', { sessionId, ip, port });
    } catch (error) {
      logger.error('Failed to connect SSH', error);
      throw error;
    }
  }

  async connectLocal(
    sessionId: string,
    cols: number,
    rows: number
  ): Promise<void> {
    try {
      await invoke<void>('connect_local', { sessionId, cols, rows } as Record<string, unknown>);
      logger.info('Local terminal connection initiated', { sessionId });
    } catch (error) {
      logger.error('Failed to connect local terminal', error);
      throw error;
    }
  }

  async disconnectSSH(sessionId: string): Promise<void> {
    try {
      logger.debug('disconnectSSH invoke', { sessionId });
      await invoke<void>('disconnect_ssh', { sessionId } as Record<string, unknown>);
      logger.info('SSH disconnection initiated', { sessionId });
    } catch (error) {
      logger.error('Failed to disconnect SSH', error);
      throw error;
    }
  }

  async disconnectLocal(sessionId: string): Promise<void> {
    try {
      await invoke<void>('disconnect_local', { sessionId });
      logger.info('Local terminal disconnection initiated', { sessionId });
    } catch (error) {
      logger.error('Failed to disconnect local terminal', error);
      throw error;
    }
  }

  async sendSSHInput(sessionId: string, input: string): Promise<void> {
    try {
      logger.debug('sendSSHInput invoke', { sessionId, inputLen: input.length });
      await invoke<void>('send_ssh_input', { sessionId, input } as Record<string, unknown>);
    } catch (error) {
      logger.error('Failed to send SSH input', error);
      throw error;
    }
  }

  async getBufferedSSHOutput(
    sessionId: string
  ): Promise<Array<{ seq: number; output: string; ts: number }>> {
    try {
      const result = await invoke<
        Array<{ seq: number; output: string; ts: number }>
      >('get_buffered_ssh_output', { sessionId } as Record<string, unknown>);
      return (
        (result as Array<{ seq: number; output: string; ts: number }>) || []
      );
    } catch (error) {
      // Surface backend failures instead of conflating them with an empty
      // buffer — callers must be able to distinguish "no buffered output"
      // from "the backend is broken".
      logger.error('Failed to get buffered SSH output', error);
      throw error;
    }
  }

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
      logger.debug('saveSession invoke', {
        addr,
        port,
        serverName,
        username,
        isFavorite,
        groupCount: groupIds?.length || 0,
        tagCount: tagIds?.length || 0,
      });

      const sessionId = await invoke<string>(
        'save_session',
        {
          addr,
          port: Math.floor(port),
          serverName,
          username,
          authType,
          privateKeyPath: privateKeyPath || null,
          isFavorite: isFavorite ?? null,
          groupIds: groupIds || null,
          tagIds: tagIds || null,
        } as Record<string, unknown>
      );

      logger.info('SSH session saved', { sessionId, serverName, addr, port });
      return sessionId;
    } catch (error) {
      logger.error('Failed to save SSH session', error);
      throw error;
    }
  }

  async toggleFavorite(id: string, isFavorite: boolean): Promise<void> {
    try {
      await invoke('toggle_favorite', { id, isFavorite });
      logger.info('Session favorite status toggled', { id, isFavorite });
    } catch (error) {
      logger.error('Failed to toggle session favorite status', error);
      throw error;
    }
  }

  async listSessions(): Promise<SavedSession[]> {
    try {
      const sessions = await invoke<SavedSession[]>('list_sessions');
      return sessions || [];
    } catch (error) {
      // Rethrow so callers can tell "no saved sessions" apart from a broken
      // backend instead of silently treating the failure as an empty list.
      logger.error('Failed to list sessions', error);
      throw error;
    }
  }

  /**
   * Resolve the stored credentials for a session. Resolves with
   * `[id, password, key_passphrase]`; either secret is null when not stored.
   */
  async getSessionCredentials(
    sessionId: string
  ): Promise<[string, string | null, string | null]> {
    try {
      return await invoke<[string, string | null, string | null]>(
        'get_session_credentials',
        { sessionId }
      );
    } catch (error) {
      logger.error('Failed to fetch session credentials', { sessionId, error });
      throw error;
    }
  }

  /**
   * Save a session together with its (possibly encrypted) credentials.
   * Resolves with the persisted session id.
   */
  async saveSessionWithCredentials(
    payload: SaveSessionWithCredentialsPayload
  ): Promise<string> {
    try {
      const id = await invoke<string>(
        'save_session_with_credentials',
        payload as unknown as Record<string, unknown>
      );
      logger.info('SSH session persisted', { id, name: payload.serverName });
      return id;
    } catch (error) {
      logger.error('Failed to persist session', error);
      throw error;
    }
  }

  /** Best-effort recency bump; failures are logged, never thrown. */
  async touchSession(id: string): Promise<void> {
    try {
      await invoke('update_session_timestamp', { id });
    } catch (error) {
      logger.error('Failed to update session timestamp', { id, error });
    }
  }

  /**
   * Import sessions from XTerminal-format text (label blocks, key=value or
   * pipe-separated lines). Resolves with the import summary.
   */
  async importXTerminal(text: string): Promise<ImportResult> {
    try {
      const result = await invoke<ImportResult>('import_xterminal_sessions', {
        text,
      } as Record<string, unknown>);
      logger.info('XTerminal import finished', {
        imported: result?.imported ?? 0,
        skipped: result?.skipped ?? 0,
        failed: result?.failed?.length ?? 0,
      });
      return result || { imported: 0, skipped: 0, failed: [] };
    } catch (error) {
      logger.error('Failed to import XTerminal sessions', error);
      throw error;
    }
  }
}

export const sessionApi = new SessionAPI();
