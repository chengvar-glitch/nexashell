import { defineStore } from 'pinia';
import { ref } from 'vue';
import { sessionApi } from './api';
import { tunnelApi } from '@/features/tunnel';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('SESSION_STORE');

export type SessionStatus =
  | 'connecting'
  | 'connected'
  | 'disconnected'
  | 'error';

export interface SessionState {
  id: string;
  tabId: string;
  type: 'ssh' | 'terminal';
  status: SessionStatus;
  createdAt: Date;
  errorMessage?: string;
  connectionParams?: {
    serverName: string;
    ip: string;
    port: number;
    username: string;
  };
}

/**
 * Non-reactive credential cache — keeps plaintext credentials OUT of the
 * reactive Pinia state (invisible to Vue DevTools / store consumers) while
 * still letting split-pane reconnects reuse the source session's credentials.
 * Entries live exactly as long as their session and are cleared on disconnect.
 */
export interface CachedCredentials {
  password?: string;
  privateKeyPath?: string | null;
  keyPassphrase?: string | null;
}

const credentialCache = new Map<string, CachedCredentials>();

export const useSessionStore = defineStore('session', () => {
  const sessions = ref<Record<string, SessionState>>({});
  const tabToSessionMap = ref<Record<string, string>>({});
  // Saved-session id (db) → runtime session id. The backend SSH/SFTP session is
  // keyed by the runtime id, so features targeting a saved session (e.g. the
  // file manager) must resolve through this map first.
  const savedToRuntime = ref<Record<string, string>>({});

  const getSession = (sessionId: string): SessionState | undefined => {
    return sessions.value[sessionId];
  };

  const hasSession = (sessionId: string): boolean => {
    return sessionId in sessions.value;
  };

  const hasSessionForTab = (tabId: string): boolean => {
    return tabId in tabToSessionMap.value;
  };

  /** Runtime session id for a saved (db) session id, if currently connected. */
  const getRuntimeSessionId = (savedSessionId: string): string | undefined => {
    return savedToRuntime.value[savedSessionId];
  };

  const createSSHSession = async (
    sessionId: string,
    tabId: string,
    serverName: string,
    ip: string,
    port: number,
    username: string,
    password: string,
    privateKeyPath?: string | null,
    keyPassphrase?: string | null,
    cols: number = 80,
    rows: number = 24,
    savedSessionId?: string | null
  ): Promise<void> => {
    if (hasSession(sessionId) || hasSessionForTab(tabId)) {
      logger.warn('Session already exists, refusing to overwrite', { sessionId, tabId });
      return;
    }
    try {
      const session: SessionState = {
        id: sessionId,
        tabId,
        type: 'ssh',
        status: 'connecting',
        createdAt: new Date(),
        connectionParams: {
          serverName,
          ip,
          port,
          username,
        },
      };

      // Credentials live in the non-reactive cache, not in session state
      credentialCache.set(sessionId, { password, privateKeyPath, keyPassphrase });

      sessions.value[sessionId] = session;
      tabToSessionMap.value[tabId] = sessionId;
      if (savedSessionId) {
        savedToRuntime.value[savedSessionId] = sessionId;
      }

      logger.debug('Creating SSH session', {
        sessionId,
        tabId,
        serverName,
        cols,
        rows,
      });

      await sessionApi.connectSSH(
        sessionId,
        ip,
        port,
        username,
        password,
        privateKeyPath,
        keyPassphrase,
        cols,
        rows
      );

      const sess = sessions.value[sessionId];
      if (sess) {
        sess.status = 'connected';
      }

      logger.info('SSH session connected', { sessionId });
    } catch (error) {
      logger.error('Failed to create SSH session', error);

      credentialCache.delete(sessionId);

      const sess = sessions.value[sessionId];
      if (sess) {
        sess.status = 'error';
        sess.errorMessage = error instanceof Error ? error.message : String(error);
      }

      throw error;
    }
  };

  const createLocalSession = async (
    sessionId: string,
    tabId: string,
    cols: number,
    rows: number
  ): Promise<void> => {
    // Same duplicate guard as createSSHSession: never silently overwrite an
    // existing session record / tab mapping with a brand-new connection.
    if (hasSession(sessionId) || hasSessionForTab(tabId)) {
      logger.warn('Session already exists, refusing to overwrite', {
        sessionId,
        tabId,
      });
      return;
    }
    try {
      const session: SessionState = {
        id: sessionId,
        tabId,
        type: 'terminal',
        status: 'connecting',
        createdAt: new Date(),
        connectionParams: {
          serverName: 'Local Terminal',
          ip: 'localhost',
          port: 0,
          username: 'local',
        },
      };

      sessions.value[sessionId] = session;
      tabToSessionMap.value[tabId] = sessionId;

      await sessionApi.connectLocal(sessionId, cols, rows);

      const sess = sessions.value[sessionId];
      if (sess) {
        sess.status = 'connected';
      }

      logger.info('Local terminal session connected', { sessionId });
    } catch (error) {
      logger.error('Failed to create local session', error);
      const sess = sessions.value[sessionId];
      if (sess) {
        sess.status = 'error';
        sess.errorMessage = error instanceof Error ? error.message : String(error);
      }
      throw error;
    }
  };

  const disconnectSession = async (sessionId: string): Promise<void> => {
    const session = sessions.value[sessionId];
    if (!session) {
      logger.warn('Session not found', { sessionId });
      return;
    }

    logger.debug('disconnectSession: initiating disconnect', {
      sessionId,
      tabId: session.tabId,
      type: session.type,
    });

    try {
      if (session.type === 'ssh') {
        await sessionApi.disconnectSSH(sessionId);
        // Tear down any active port-forwarding tunnels for this session.
        // Best-effort: tunnels are torn down by the backend when the SSH
        // session itself drops, so a failure here must not block the
        // disconnect, BUT it must also not orphan live connections.
        try {
          await tunnelApi.stopSessionTunnels(sessionId);
        } catch (error) {
          logger.error('Failed to stop session tunnels', error);
        }
      } else if (session.type === 'terminal') {
        await sessionApi.disconnectLocal(sessionId);
      }
    } catch (error) {
      logger.error('Failed to disconnect session on backend', error);
      // The backend did not actually tear the connection down. Deleting the
      // local record here would leave an orphaned live connection with no
      // way to manage or close it — so keep the record and surface the
      // failure instead of silently dropping state.
      session.status = 'error';
      session.errorMessage =
        error instanceof Error ? error.message : String(error);
      throw error;
    }

    // Success — only now remove local state.
    delete sessions.value[sessionId];
    delete tabToSessionMap.value[session.tabId];
    for (const [savedId, runtimeId] of Object.entries(savedToRuntime.value)) {
      if (runtimeId === sessionId) delete savedToRuntime.value[savedId];
    }
    credentialCache.delete(sessionId);

    logger.info('disconnectSession: removed session state locally', {
      sessionId,
    });
  };

  const disconnectSessions = async (ids: string[]): Promise<void> => {
    const errors: Error[] = [];
    for (const id of ids) {
      try {
        await disconnectSession(id);
      } catch (error) {
        logger.error('Error disconnecting session', { id, error });
        if (error instanceof Error) {
          errors.push(error);
        }
      }
    }
    if (errors.length > 0) {
      logger.warn('Some sessions failed to disconnect', { count: errors.length });
      throw new Error(
        `Failed to disconnect ${errors.length} session(s): ${errors[0]?.message ?? 'unknown'}`
      );
    }
  };

  const updateSessionStatus = (sessionId: string, status: SessionStatus) => {
    const session = sessions.value[sessionId];
    if (session) {
      session.status = status;
    }
  };

  const getCachedCredentials = (sessionId: string): CachedCredentials | undefined => {
    return credentialCache.get(sessionId);
  };

  /**
   * Pre-seeds credentials for a session that hasn't been created yet —
   * used by split-pane flow so the freshly mounted terminal can resolve
   * credentials at connect time without them ever entering reactive state.
   */
  const cacheCredentials = (sessionId: string, creds: CachedCredentials): void => {
    credentialCache.set(sessionId, creds);
  };

  const cleanupAllSessions = async (): Promise<void> => {
    const sessionIds = Object.keys(sessions.value);
    for (const sessionId of sessionIds) {
      try {
        await disconnectSession(sessionId);
      } catch (error) {
        logger.error('Error cleaning up session', error);
      }
    }
  };

  const reset = () => {
    sessions.value = {};
    tabToSessionMap.value = {};
    savedToRuntime.value = {};
    credentialCache.clear();
  };

  return {
    sessions,
    tabToSessionMap,
    savedToRuntime,
    getSession,
    hasSession,
    getRuntimeSessionId,
    createSSHSession,
    createLocalSession,
    disconnectSession,
    disconnectSessions,
    getCachedCredentials,
    cacheCredentials,
    updateSessionStatus,
    cleanupAllSessions,
    reset,
  };
});
