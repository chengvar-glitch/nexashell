/**
 * Session Store (Pinia)
 *
 * Centralized, type-safe state management for SSH sessions and terminal
 * connections. Responsibilities:
 * - Maintain canonical session state used by UI components
 * - Orchestrate lifecycle operations (create, disconnect, cleanup)
 * - Delegate transport-level operations to `sessionApi` (Tauri IPC)
 *
 * Design notes:
 * - The store intentionally separates state and transport: `sessionApi`
 *   handles Tauri RPCs while the store manages local in-memory state.
 * - This separation makes the store easier to unit-test and prevents
 *   UI components from needing to know about Tauri specifics.
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { sessionApi } from '@/features/session';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('SESSION_STORE');

/**
 * Session lifecycle status enum (string union for readability in devtools)
 */
export type SessionStatus =
  | 'connecting'
  | 'connected'
  | 'disconnected'
  | 'error';

/**
 * SessionState describes a single terminal/SSH session's canonical state.
 * @property id Unique session identifier (used by backend and store)
 * @property tabId Associated UI tab identifier
 * @property type Session type: 'ssh' for remote, 'terminal' for local shells
 * @property status Current lifecycle status
 * @property createdAt Timestamp when the session was created
 * @property errorMessage Optional human-readable error details when in 'error' state
 * @property connectionParams Optional connection parameters used to establish the session
 */
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
 * Pinia session store
 *
 * Usage:
 * ```typescript
 * const sessionStore = useSessionStore();
 *
 * // Create session
 * await sessionStore.createSSHSession(params);
 *
 * // Disconnect session
 * await sessionStore.disconnectSession(sessionId);
 *
 * // Query sessions
 * const session = sessionStore.getSession(sessionId);
 * const tabSession = sessionStore.getSessionByTabId(tabId);
 * ```
 */
export const useSessionStore = defineStore('session', () => {
  // State
  const sessions = ref<Map<string, SessionState>>(new Map());
  const tabToSessionMap = ref<Map<string, string>>(new Map());

  // Getters
  /**
   * Get all sessions
   */
  const allSessions = computed(() => {
    return Array.from(sessions.value.values());
  });

  /**
   * Get session by ID
   */
  const getSession = (sessionId: string): SessionState | undefined => {
    return sessions.value.get(sessionId);
  };

  /**
   * Get session by tab ID
   */
  const getSessionByTabId = (tabId: string): SessionState | undefined => {
    const sessionId = tabToSessionMap.value.get(tabId);
    return sessionId ? sessions.value.get(sessionId) : undefined;
  };

  /**
   * Check if session exists
   */
  const hasSession = (sessionId: string): boolean => {
    return sessions.value.has(sessionId);
  };

  /**
   * Check if tab has an active session
   */
  const hasSessionForTab = (tabId: string): boolean => {
    return tabToSessionMap.value.has(tabId);
  };

  /**
   * Get session statistics
   */
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

  /**
   * Get active session count
   */
  const activeSessionCount = computed(() => {
    return sessionStats.value.connected;
  });

  // Actions
  /**
   * Create an SSH session and request the backend to establish the connection.
   * This action will:
   * - Add an entry to the local store immediately with status 'connecting'
   * - Call `sessionApi.connectSSH(...)` to trigger the backend connection
   * - Update the session status to 'connected' on success or 'error' on failure
   *
   * @param sessionId Unique session identifier
   * @param tabId UI tab identifier associated with this session
   * @param serverName Human-friendly server name
   * @param ip Remote host
   * @param port Remote port
   * @param username Login username
   * @param password Password or empty string if using key-based auth
   * @param cols Initial terminal columns
   * @param rows Initial terminal rows
   */
  const createSSHSession = async (
    sessionId: string,
    tabId: string,
    serverName: string,
    ip: string,
    port: number,
    username: string,
    password: string,
    cols: number,
    rows: number
  ): Promise<void> => {
    try {
      // Create session state
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

      sessions.value.set(sessionId, session);
      tabToSessionMap.value.set(tabId, sessionId);

      logger.debug('Creating SSH session', {
        sessionId,
        tabId,
        serverName,
        cols,
        rows,
      });

      // Call API to establish connection (Tauri 2.0+ optimized, no serverName param)
      await sessionApi.connectSSH(
        sessionId,
        ip,
        port,
        username,
        password,
        cols,
        rows
      );

      // Update status
      const sess = sessions.value.get(sessionId);
      if (sess) {
        sess.status = 'connected';
      }

      logger.info('SSH session connected', { sessionId });
    } catch (error) {
      logger.error('Failed to create SSH session', error);

      // Update status to error
      const sess = sessions.value.get(sessionId);
      if (sess) {
        sess.status = 'error';
        sess.errorMessage =
          error instanceof Error ? error.message : String(error);
      }

      throw error;
    }
  };

  /**
   * Disconnect a session by its session ID.
   * Behavior:
   * - Calls `sessionApi.disconnectSSH` to instruct the backend to tear down resources
   * - Marks the session as 'disconnected'
   * - Removes session state mappings from the store
   *
   * Note: the backend call is attempted first but errors do not prevent local cleanup.
   */
  const disconnectSession = async (sessionId: string): Promise<void> => {
    try {
      const session = sessions.value.get(sessionId);
      if (!session) {
        logger.warn('Session not found', { sessionId });
        return;
      }

      logger.debug('disconnectSession: initiating disconnect', {
        sessionId,
        tabId: session.tabId,
      });
      // Call API to disconnect FIRST, before removing local state
      // This prevents the backend from failing due to missing session data
      if (session.type === 'ssh') {
        try {
          await sessionApi.disconnectSSH(sessionId);
          logger.debug('disconnectSession: backend disconnect returned', {
            sessionId,
          });
        } catch (error) {
          logger.error('Failed to disconnect SSH session on backend', error);
          // Continue with local cleanup even if backend disconnect fails
        }
      }

      // Update status
      session.status = 'disconnected';

      // Remove from mappings AFTER backend cleanup completes
      sessions.value.delete(sessionId);
      tabToSessionMap.value.delete(session.tabId);

      logger.info('disconnectSession: removed session state locally', {
        sessionId,
      });

      logger.info('Session disconnected', { sessionId });
    } catch (error) {
      logger.error('Failed to disconnect session', error);
      throw error;
    }
  };

  /**
   * Disconnect session by tab ID
   */
  const disconnectByTabId = async (tabId: string): Promise<void> => {
    const sessionId = tabToSessionMap.value.get(tabId);
    if (!sessionId) {
      logger.warn('No session found for tab', { tabId });
      return;
    }
    await disconnectSession(sessionId);
  };

  /**
   * Update session status
   */
  const updateSessionStatus = (sessionId: string, status: SessionStatus) => {
    const session = sessions.value.get(sessionId);
    if (session) {
      session.status = status;
    }
  };

  /**
   * Set session error
   */
  const setSessionError = (sessionId: string, errorMessage: string) => {
    const session = sessions.value.get(sessionId);
    if (session) {
      session.status = 'error';
      session.errorMessage = errorMessage;
    }
  };

  /**
   * Clean up all sessions
   */
  const cleanupAllSessions = async (): Promise<void> => {
    const sessionIds = Array.from(sessions.value.keys());
    for (const sessionId of sessionIds) {
      try {
        await disconnectSession(sessionId);
      } catch (error) {
        logger.error('Error cleaning up session', error);
      }
    }
  };

  /**
   * Reset store (for testing)
   */
  const reset = () => {
    sessions.value.clear();
    tabToSessionMap.value.clear();
  };

  return {
    // State
    sessions,
    tabToSessionMap,

    // Getters
    allSessions,
    sessionStats,
    activeSessionCount,

    // Methods
    getSession,
    getSessionByTabId,
    hasSession,
    hasSessionForTab,

    // Actions
    createSSHSession,
    disconnectSession,
    disconnectByTabId,
    updateSessionStatus,
    setSessionError,
    cleanupAllSessions,
    reset,
  };
});
