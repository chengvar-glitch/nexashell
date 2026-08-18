import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '@/core/utils/logger';
import type { SavedSession } from './types';

const logger = createLogger('SESSION_API');

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
}

export const sessionApi = new SessionAPI();
