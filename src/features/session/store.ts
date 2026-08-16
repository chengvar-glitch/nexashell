import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { sessionApi } from '@/features/session';
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

  const allSessions = computed(() => {
    return Object.values(sessions.value);
  });

  const getSession = (sessionId: string): SessionState | undefined => {
    return sessions.value[sessionId];
  };

  const getSessionByTabId = (tabId: string): SessionState | undefined => {
    const sessionId = tabToSessionMap.value[tabId];
    return sessionId ? sessions.value[sessionId] : undefined;
  };

  const hasSession = (sessionId: string): boolean => {
    return sessionId in sessions.value;
  };

  const hasSessionForTab = (tabId: string): boolean => {
    return tabId in tabToSessionMap.value;
  };

  const sessionStats = computed(() => {
    const allSess = allSessions.value;
    return {
      total: allSess.length,
      connected: allSess.filter(s => s.status === 'connected').length,
      connecting: allSess.filter(s => s.status === 'connecting').length,
      error: allSess.filter(s => s.status === 'error').length,
      disconnected: allSess.filter(s => s.status === 'disconnected').length,
    };
  });

  const activeSessionCount = computed(() => {
    return sessionStats.value.connected;
  });

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
    rows: number = 24
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
    try {
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

      if (session.type === 'ssh') {
        try {
          await sessionApi.disconnectSSH(sessionId);
        } catch (error) {
          logger.error('Failed to disconnect SSH session on backend', error);
        }
        // Tear down any active port-forwarding tunnels for this session.
        try {
          await tunnelApi.stopSessionTunnels(sessionId);
        } catch (error) {
          logger.error('Failed to stop session tunnels', error);
        }
      } else if (session.type === 'terminal') {
        try {
          await sessionApi.disconnectLocal(sessionId);
        } catch (error) {
          logger.error('Failed to disconnect local session on backend', error);
        }
      }

      delete sessions.value[sessionId];
      delete tabToSessionMap.value[session.tabId];
      credentialCache.delete(sessionId);

      logger.info('disconnectSession: removed session state locally', {
        sessionId,
      });
    } catch (error) {
      logger.error('Failed to disconnect session', error);
      throw error;
    }
  };

  const disconnectByTabId = async (tabId: string): Promise<void> => {
    const sessionId = tabToSessionMap.value[tabId];
    if (!sessionId) {
      logger.warn('No session found for tab', { tabId });
      return;
    }
    await disconnectSession(sessionId);
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
    }
  };

  const updateSessionStatus = (sessionId: string, status: SessionStatus) => {
    const session = sessions.value[sessionId];
    if (session) {
      session.status = status;
    }
  };

  const setSessionError = (sessionId: string, errorMessage: string) => {
    const session = sessions.value[sessionId];
    if (session) {
      session.status = 'error';
      session.errorMessage = errorMessage;
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
    credentialCache.clear();
  };

  return {
    sessions,
    tabToSessionMap,
    allSessions,
    sessionStats,
    activeSessionCount,
    getSession,
    getSessionByTabId,
    hasSession,
    hasSessionForTab,
    createSSHSession,
    createLocalSession,
    disconnectSession,
    disconnectByTabId,
    disconnectSessions,
    getCachedCredentials,
    cacheCredentials,
    updateSessionStatus,
    setSessionError,
    cleanupAllSessions,
    reset,
  };
});
