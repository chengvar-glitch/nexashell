import { listen, Event } from '@tauri-apps/api/event';

/**
 * SSH Event Manager
 *
 * Handles SSH output events emitted from the Tauri backend and distributes them
 * to registered listeners. Includes a buffering mechanism for events received
 * before a listener is attached.
 */
class SSHEventManager {
  private static instance: SSHEventManager;
  private eventListeners: Map<string, ((data: string) => void)[]> = new Map();
  private eventBuffer: Map<string, string[]> = new Map(); // Buffer to store events received before high-level listeners register

  private constructor() {
    this.setupGlobalListener();
  }

  public static getInstance(): SSHEventManager {
    if (!SSHEventManager.instance) {
      SSHEventManager.instance = new SSHEventManager();
    }
    return SSHEventManager.instance;
  }

  /**
   * Initialize global SSH output event listener (Tauri IPC)
   */
  private async setupGlobalListener() {
    console.log('[SSH_EVENT_MGR] Setting up global SSH output listener');

    await listen('any', (event: Event<unknown>) => {
      // Check if it's an ssh-output event
      if (event.event.startsWith('ssh-output-')) {
        const sessionId = event.event.substring('ssh-output-'.length);
        const payload = event.payload as unknown;
        let outputStr: string;

        if (payload && typeof payload === 'object' && 'output' in payload) {
          const p = payload as Record<string, unknown>;
          outputStr = String(p['output']);
        } else {
          outputStr = String(payload ?? '');
        }

        console.log(
          '[SSH_EVENT_MGR] Received SSH output for session:',
          sessionId,
          'payload:',
          outputStr
        );

        // Add event to buffer
        if (!this.eventBuffer.has(sessionId)) {
          this.eventBuffer.set(sessionId, []);
        }
        this.eventBuffer.get(sessionId)!.push(outputStr);

        // Notify registered listeners immediately
        const listeners = this.eventListeners.get(sessionId);
        if (listeners) {
          listeners.forEach(listener => {
            try {
              listener(outputStr);
            } catch (error) {
              console.error(
                '[SSH_EVENT_MGR] Error in SSH output listener:',
                error
              );
            }
          });
        }
      }
    }).catch(error => {
      console.error('[SSH_EVENT_MGR] Failed to set up global listener:', error);
    });
  }

  /**
   * Subscribe to SSH output events for a specific session
   */
  public subscribe(
    sessionId: string,
    callback: (data: string) => void
  ): () => void {
    console.log(
      '[SSH_EVENT_MGR] Subscribing to SSH output for session:',
      sessionId
    );

    if (!this.eventListeners.has(sessionId)) {
      this.eventListeners.set(sessionId, []);
    }

    const listeners = this.eventListeners.get(sessionId)!;
    listeners.push(callback);

    // Flush all buffered events to the new listener
    const bufferedEvents = this.eventBuffer.get(sessionId) || [];
    bufferedEvents.forEach(data => {
      try {
        callback(data);
      } catch (error) {
        console.error(
          '[SSH_EVENT_MGR] Error in buffered SSH output callback:',
          error
        );
      }
    });

    // Return unsubscription cleanup function
    return () => {
      const index = listeners.indexOf(callback);
      if (index > -1) {
        listeners.splice(index, 1);
      }

      // If no listeners remain, consider cleanup
      if (listeners.length === 0) {
        this.eventListeners.delete(sessionId);
        this.eventBuffer.set(sessionId, []); // Clear buffer for now
      }
    };
  }

  /**
   * Get count of buffered events (primarily for debugging)
   */
  public getBufferSize(sessionId: string): number {
    return this.eventBuffer.get(sessionId)?.length || 0;
  }
}

export const sshEventManager = SSHEventManager.getInstance();
