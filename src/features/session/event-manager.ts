import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface OutputPayload {
  seq: number;
  output: string;
  ts: number;
}

class SSHEventManager {
  private static instance: SSHEventManager;
  private eventListeners: Map<string, ((data: string) => void)[]> = new Map();
  private eventBuffer: Map<string, string[]> = new Map();
  private unlistenFns: Map<string, UnlistenFn> = new Map();
  private pendingListeners: Map<string, Promise<UnlistenFn>> = new Map();
  private static readonly MAX_BUFFER_SIZE = 1000;

  private constructor() {}

  public static getInstance(): SSHEventManager {
    if (!SSHEventManager.instance) {
      SSHEventManager.instance = new SSHEventManager();
    }
    return SSHEventManager.instance;
  }

  private async ensureSessionListener(sessionId: string) {
    if (this.unlistenFns.has(sessionId)) return;

    const existing = this.pendingListeners.get(sessionId);
    if (existing) {
      try {
        await existing;
      } catch {
        // First caller already logged the setup failure
      }
      return;
    }

    const eventName = `ssh-output-${sessionId}`;
    const pending = (async () => {
      const unlisten = await listen<OutputPayload>(eventName, (event) => {
        const payload = event.payload;
        const outputStr = String(payload.output ?? '');

        if (!this.eventBuffer.has(sessionId)) {
          this.eventBuffer.set(sessionId, []);
        }
        const buf = this.eventBuffer.get(sessionId)!;
        buf.push(outputStr);
        if (buf.length > SSHEventManager.MAX_BUFFER_SIZE) {
          buf.splice(0, buf.length - SSHEventManager.MAX_BUFFER_SIZE);
        }

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
      });
      this.unlistenFns.set(sessionId, unlisten);
      return unlisten;
    })();
    this.pendingListeners.set(sessionId, pending);
    try {
      await pending;
    } catch (error) {
      console.error(
        '[SSH_EVENT_MGR] Failed to set up listener for session:',
        sessionId,
        error
      );
    } finally {
      this.pendingListeners.delete(sessionId);
    }
  }

  public async subscribe(
    sessionId: string,
    callback: (data: string) => void
  ): Promise<() => void> {
    await this.ensureSessionListener(sessionId);

    if (!this.eventListeners.has(sessionId)) {
      this.eventListeners.set(sessionId, []);
    }

    const listeners = this.eventListeners.get(sessionId)!;
    listeners.push(callback);

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

    return () => {
      const index = listeners.indexOf(callback);
      if (index > -1) {
        listeners.splice(index, 1);
      }

      if (listeners.length === 0) {
        this.eventListeners.delete(sessionId);
        this.eventBuffer.delete(sessionId);

        const unlisten = this.unlistenFns.get(sessionId);
        if (unlisten) {
          unlisten();
          this.unlistenFns.delete(sessionId);
        }
      }
    };
  }

  public getBufferSize(sessionId: string): number {
    return this.eventBuffer.get(sessionId)?.length || 0;
  }
}

export const sshEventManager = SSHEventManager.getInstance();
