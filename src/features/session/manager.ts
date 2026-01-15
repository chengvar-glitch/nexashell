/**
 * Session Manager (Legacy Compatibility Layer)
 *
 * DEPRECATED: Use useSessionStore from Pinia instead.
 *
 * This class is kept for backward compatibility during migration.
 * New code should use the Pinia store directly:
 *
 * ```typescript
 * import { useSessionStore } from '@/features/session';
 * const sessionStore = useSessionStore();
 * await sessionStore.createSSHSession(...);
 * ```
 *
 * This will be removed in v1.0.0
 */

import { useSessionStore } from '@/features/session';

/**
 * @deprecated Use useSessionStore() from Pinia instead
 *
 * Delegates all operations to the Pinia store.
 */
class SessionManager {
  /**
   * @deprecated Use sessionStore.createSSHSession()
   */
  async createSSHSession(
    sessionId: string,
    tabId: string,
    serverName: string,
    ip: string,
    port: number,
    username: string,
    password: string,
    cols: number = 80,
    rows: number = 24
  ): Promise<void> {
    const store = useSessionStore();
    return store.createSSHSession(
      sessionId,
      tabId,
      serverName,
      ip,
      port,
      username,
      password,
      cols,
      rows
    );
  }

  /**
   * @deprecated Use sessionStore.disconnectSession()
   */
  async disconnectSession(sessionId: string): Promise<void> {
    const store = useSessionStore();
    return store.disconnectSession(sessionId);
  }

  /**
   * @deprecated Use sessionStore.getSessionByTabId()
   */
  disconnectByTabId(tabId: string): Promise<void> {
    const store = useSessionStore();
    return store.disconnectByTabId(tabId);
  }

  /**
   * @deprecated Use sessionStore.getSession()
   */
  getSession(sessionId: string) {
    const store = useSessionStore();
    return store.getSession(sessionId);
  }

  /**
   * @deprecated Use sessionStore.getSessionByTabId()
   */
  getSessionByTabId(tabId: string) {
    const store = useSessionStore();
    return store.getSessionByTabId(tabId);
  }

  /**
   * @deprecated Use sessionStore.allSessions
   */
  getAllSessions() {
    const store = useSessionStore();
    return store.allSessions;
  }

  /**
   * @deprecated Use sessionStore.hasSession()
   */
  hasSession(sessionId: string): boolean {
    const store = useSessionStore();
    return store.hasSession(sessionId);
  }

  /**
   * @deprecated Use sessionStore.hasSessionForTab()
   */
  hasSessionForTab(tabId: string): boolean {
    const store = useSessionStore();
    return store.hasSessionForTab(tabId);
  }

  /**
   * @deprecated Use sessionStore.cleanupAllSessions()
   */
  async cleanupAllSessions(): Promise<void> {
    const store = useSessionStore();
    return store.cleanupAllSessions();
  }

  /**
   * @deprecated Use sessionStore.activeSessionCount
   */
  getActiveSessionCount(): number {
    const store = useSessionStore();
    return store.activeSessionCount;
  }
}

// Create singleton instance for backward compatibility
export const sessionManager = new SessionManager();

// Export the SessionManager type for legacy usage.
// NOTE: This type is provided for backward compatibility only.
// Prefer using the Pinia store `useSessionStore()` in new code.
export type { SessionManager };
