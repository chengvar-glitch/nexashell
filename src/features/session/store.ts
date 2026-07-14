import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { sessionApi } from '@/features/session';
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

      sessions.value = { ...sessions.value, [sessionId]: session };
      tabToSessionMap.value = { ...tabToSessionMap.value, [tabId]: sessionId };

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
        sessions.value = { ...sessions.value, [sessionId]: { ...sess, status: 'connected' } };
      }

      logger.info('SSH session connected', { sessionId });
    } catch (error) {
      logger.error('Failed to create SSH session', error);

      const sess = sessions.value[sessionId];
      if (sess) {
        sessions.value = {
          ...sessions.value,
          [sessionId]: {
            ...sess,
            status: 'error',
            errorMessage:
              error instanceof Error ? error.message : String(error),
          },
        };
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

      sessions.value = { ...sessions.value, [sessionId]: session };
      tabToSessionMap.value = { ...tabToSessionMap.value, [tabId]: sessionId };

      await sessionApi.connectLocal(sessionId, cols, rows);

      const sess = sessions.value[sessionId];
      if (sess) {
        sessions.value = { ...sessions.value, [sessionId]: { ...sess, status: 'connected' } };
      }

      logger.info('Local terminal session connected', { sessionId });
    } catch (error) {
      logger.error('Failed to create local session', error);
      const sess = sessions.value[sessionId];
      if (sess) {
        sessions.value = {
          ...sessions.value,
          [sessionId]: {
            ...sess,
            status: 'error',
            errorMessage:
              error instanceof Error ? error.message : String(error),
          },
        };
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
      } else if (session.type === 'terminal') {
        try {
          await sessionApi.disconnectLocal(sessionId);
        } catch (error) {
          logger.error('Failed to disconnect local session on backend', error);
        }
      }

      sessions.value = {
        ...sessions.value,
        [sessionId]: { ...session, status: 'disconnected' },
      };

      const restSessions: Record<string, SessionState> = {};
      for (const key of Object.keys(sessions.value)) {
        if (key !== sessionId) restSessions[key] = sessions.value[key];
      }
      const restTabs: Record<string, string> = {};
      for (const key of Object.keys(tabToSessionMap.value)) {
        if (key !== session.tabId) restTabs[key] = tabToSessionMap.value[key];
      }
      sessions.value = restSessions;
      tabToSessionMap.value = restTabs;

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

  const updateSessionStatus = (sessionId: string, status: SessionStatus) => {
    const session = sessions.value[sessionId];
    if (session) {
      sessions.value = { ...sessions.value, [sessionId]: { ...session, status } };
    }
  };

  const setSessionError = (sessionId: string, errorMessage: string) => {
    const session = sessions.value[sessionId];
    if (session) {
      sessions.value = {
        ...sessions.value,
        [sessionId]: { ...session, status: 'error' as const, errorMessage },
      };
    }
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
    updateSessionStatus,
    setSessionError,
    cleanupAllSessions,
    reset,
  };
});
