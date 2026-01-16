<script setup lang="ts">
import { onMounted, onUnmounted, ref, nextTick, watch, onActivated } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import '@xterm/xterm/css/xterm.css';
import { useSessionStore } from '@/features/session';
import { sessionApi } from '@/features/session';
import { createLogger } from '@/core/utils/logger';
import { listen, UnlistenFn, emit } from '@tauri-apps/api/event';

const logger = createLogger('TERMINAL_VIEW');

// Terminal configuration constants
const TERMINAL_CONFIG = {
  SCROLLBACK: 80000,
  FONT_SIZE: 14,
  FONT_FAMILY: 'Monaco, Menlo, Ubuntu Mono, monospace',
  ROWS: 24,
  COLS: 80,
  CURSOR_BLINK: true,
  CURSOR_STYLE: 'block' as const,
  THEME: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
  },
};

const INPUT_BUFFER_CONFIG = {
  FLUSH_THRESHOLD: 32,
  FLUSH_DELAY_MS: 20,
};

const LATENCY_THRESHOLD_MS = 100;

/**
 * Props interface
 * Note: serverName is stored in session.connectionParams, no need to pass separately
 */
interface Props {
  sessionId?: string;
  ip?: string;
  port?: number;
  username?: string;
  password?: string;
}

const props = withDefaults(defineProps<Props>(), {
  sessionId: '',
  ip: '',
  port: 22,
  username: '',
  password: '',
});

const sessionStore = useSessionStore();

const terminalRef = ref<HTMLElement>();
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;

// Output deduplication tracking
let lastSeq = 0;
let unlistenFn: UnlistenFn | null = null;

/**
 * Establish SSH connection via session store and API
 */
const connectSSH = async (cols: number, rows: number): Promise<void> => {
  if (!props.sessionId) {
    throw new Error('sessionId is required');
  }

  if (sessionStore.hasSession(props.sessionId)) {
    return;
  }

  try {
    // Get session info from store (includes serverName)
    const session = sessionStore.getSession(props.sessionId);
    const serverName =
      session?.connectionParams?.serverName || props.ip || 'Unknown';

    await sessionStore.createSSHSession(
      props.sessionId,
      props.sessionId,
      serverName,
      props.ip || '',
      props.port || 22,
      props.username || '',
      props.password || '',
      cols,
      rows
    );

    // ✅ Get buffered initial output (welcome banner, etc.)
    // Wait for backend to complete initial buffering phase (2 seconds)
    // This ensures SSH welcome banner and login prompts are fully captured
    await new Promise(resolve => setTimeout(resolve, 2100));

    const bufferedOutput = await sessionApi.getBufferedSSHOutput(
      props.sessionId
    );
    if (terminal && bufferedOutput.length > 0) {
      logger.info('Writing buffered SSH output to terminal', {
        chunks: bufferedOutput.length,
      });
      for (const chunk of bufferedOutput) {
        terminal.write(chunk.output);
        lastSeq = Math.max(lastSeq, chunk.seq);
      }
    } else {
      logger.debug('No buffered SSH output available');
    }
  } catch (error) {
    logger.error('SSH connection failed', error);
    throw error;
  }
};

/**
 * Disconnect SSH session
 */
const disconnectSSH = async (): Promise<void> => {
  if (!props.sessionId) return;

  try {
    await sessionApi.disconnectSSH(props.sessionId);
    sessionStore.disconnectSession(props.sessionId);
  } catch (error) {
    logger.error('SSH disconnect failed', error);
  }
};

/**
 * Cleanup terminal resources
 */
const cleanupResources = async (): Promise<void> => {
  if (props.sessionId) {
    await disconnectSSH();
  }

  terminal?.dispose();

  if (unlistenFn) {
    try {
      await unlistenFn();
      unlistenFn = null;
    } catch (e) {
      logger.error('Event unlisten failed', e);
    }
  }
};

/**
 * Expose cleanup method to parent
 */
defineExpose({
  cleanupResources,
});

/**
 * Component lifecycle
 */
onMounted(async () => {
  logger.info('Terminal component mounted', { sessionId: props.sessionId });

  if (!terminalRef.value) return;

  // Initialize xterm.js terminal
  terminal = new Terminal({
    scrollback: TERMINAL_CONFIG.SCROLLBACK,
    fontSize: TERMINAL_CONFIG.FONT_SIZE,
    fontFamily: TERMINAL_CONFIG.FONT_FAMILY,
    rows: TERMINAL_CONFIG.ROWS,
    cols: TERMINAL_CONFIG.COLS,
    cursorBlink: TERMINAL_CONFIG.CURSOR_BLINK,
    cursorStyle: TERMINAL_CONFIG.CURSOR_STYLE,
    theme: TERMINAL_CONFIG.THEME,
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  // Use WebGL renderer for better performance
  try {
    terminal.loadAddon(new WebglAddon());
  } catch {
    logger.warn('WebGL addon unavailable, using standard renderer');
  }

  terminal.open(terminalRef.value);

  // Use ResizeObserver for robust layout management
  const resizeObserver = new ResizeObserver(() => {
    if (fitAddon) {
      try {
        fitAddon.fit();
      } catch (e) {
        logger.error('Fit error', e);
      }
    }
  });

  if (terminalRef.value) {
    resizeObserver.observe(terminalRef.value);
  }

  await nextTick();
  fitAddon.fit();
  terminal.focus();

  // Handle terminal resize and notify backend
  terminal.onResize(({ cols, rows }) => {
    if (props.sessionId) {
      emit(`ssh-resize-${props.sessionId}`, { cols, rows });
    }
  });

  // Initial connection is handled when a valid `sessionId` is provided

  // Handle activation when switching back to this tab (KeepAlive)
  onActivated(() => {
    nextTick(() => {
      if (fitAddon) {
        fitAddon.fit();
        terminal?.focus();

        // Force a sync after activation
        if (terminal && props.sessionId) {
          emit(`ssh-resize-${props.sessionId}`, {
            cols: terminal.cols,
            rows: terminal.rows,
          });
        }
      }
    });
  });

  // Handle window resize
  const handleResize = (): void => {
    if (fitAddon) {
      fitAddon.fit();
    }
  };
  window.addEventListener('resize', handleResize);

  /**
   * Setup SSH output event listener before connection
   */
  const setupSSHOutputListener = async (sessionId: string): Promise<void> => {
    if (unlistenFn) {
      await unlistenFn();
    }

    unlistenFn = await listen(`ssh-output-${sessionId}`, (event: any) => {
      try {
        const payload = event.payload as
          | { seq?: number; output?: string; ts?: number }
          | undefined;

        if (
          payload?.seq !== undefined &&
          payload.output !== undefined &&
          terminal
        ) {
          if (payload.seq > lastSeq) {
            terminal.write(String(payload.output));
            lastSeq = payload.seq;

            // Monitor high latency
            if (payload.ts && Date.now() - payload.ts > LATENCY_THRESHOLD_MS) {
              logger.debug('High latency in SSH output', {
                latency: Date.now() - payload.ts,
              });
            }
          }
        }
      } catch (e) {
        logger.error('Terminal write failed', e);
      }
    });
  };

  /**
   * Watch sessionId changes and connect/disconnect accordingly
   */
  watch(
    () => props.sessionId,
    async (newSessionId, oldSessionId) => {
      if (newSessionId && newSessionId !== oldSessionId) {
        lastSeq = 0;

        try {
          // ✅ Register listener BEFORE connection to catch welcome message
          await setupSSHOutputListener(newSessionId);

          if (terminal) {
            // First, ensure we have the correct size before connecting
            if (fitAddon) {
              fitAddon.fit();
            }

            await connectSSH(terminal.cols, terminal.rows);

            // Re-sync after a short delay to ensure backend is ready and listener is active
            setTimeout(() => {
              if (fitAddon && props.sessionId === newSessionId) {
                fitAddon.fit();
                emit(`ssh-resize-${newSessionId}`, {
                  cols: terminal!.cols,
                  rows: terminal!.rows,
                });
              }
            }, 500);
          }
        } catch (error) {
          logger.error('SSH connection failed', error);
        }
      }
    },
    { immediate: true }
  );

  /**
   * Handle terminal input
   */
  // Key sequence mapping for terminal input
  const KEY_SEQUENCES: Record<string, string> = {
    Enter: '\r',
    Backspace: '\x7f',
    Tab: '\t',
    Escape: '\x1b',
    ArrowUp: '\x1b[A',
    ArrowDown: '\x1b[B',
    ArrowRight: '\x1b[C',
    ArrowLeft: '\x1b[D',
    Home: '\x1b[H',
    End: '\x1b[F',
    PageUp: '\x1b[5~',
    PageDown: '\x1b[6~',
    Insert: '\x1b[2~',
    Delete: '\x1b[3~',
  };

  // Map raw keyboard events to terminal byte sequences
  function mapKeyEventToSequence(
    key: string,
    domEvent?: KeyboardEvent
  ): string {
    if (domEvent) {
      domEvent.preventDefault();
      domEvent.stopPropagation();

      // Ctrl+<char> -> control code
      if (
        domEvent.ctrlKey &&
        !domEvent.altKey &&
        !domEvent.metaKey &&
        key.length === 1
      ) {
        const code = key.toUpperCase().charCodeAt(0);
        if (code >= 64 && code <= 95) {
          return String.fromCharCode(code - 64);
        }
      }

      // Alt -> ESC prefix
      if (domEvent.altKey && !domEvent.ctrlKey && !domEvent.metaKey) {
        return '\x1b' + (key.length === 1 ? key : '');
      }
    }

    return KEY_SEQUENCES[key] ?? key;
  }

  // Input buffering for batching
  let inputBuffer = '';
  let inputFlushTimeout: ReturnType<typeof setTimeout> | null = null;

  const flushInput = async (): Promise<void> => {
    if (!inputBuffer || !props.sessionId) return;

    const data = inputBuffer;
    inputBuffer = '';

    try {
      await emit(`ssh-input-${props.sessionId}`, { input: data });
    } catch (error) {
      logger.error('Input emit failed', error);
    }
  };

  terminal.onKey(async (event: { key: string; domEvent?: KeyboardEvent }) => {
    const seq = mapKeyEventToSequence(
      event.key,
      event.domEvent as KeyboardEvent | undefined
    );
    const hasSession =
      props.sessionId && sessionStore.hasSession(props.sessionId);

    if (hasSession) {
      inputBuffer += seq;

      if (inputBuffer.length > INPUT_BUFFER_CONFIG.FLUSH_THRESHOLD) {
        clearTimeout(inputFlushTimeout!);
        await flushInput();
      } else {
        clearTimeout(inputFlushTimeout!);
        inputFlushTimeout = setTimeout(
          flushInput,
          INPUT_BUFFER_CONFIG.FLUSH_DELAY_MS
        );
      }
    } else if (seq.length === 1 && seq >= ' ') {
      // No active session: echo printable characters locally
      terminal?.write(seq);
    }
  });

  /**
   * Cleanup on unmount
   * With KeepAlive, this only fires when the tab is truly closed or the cache is purged.
   */
  onUnmounted(async () => {
    window.removeEventListener('resize', handleResize);
    resizeObserver.disconnect();
    await cleanupResources();
  });
});
</script>

<template>
  <div class="terminal-view">
    <div ref="terminalRef" class="terminal-container" />
  </div>
</template>

<style scoped>
.terminal-view {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: #1e1e1e;
  padding: 10px;
  box-sizing: border-box;
}

.terminal-container {
  width: 100%;
  height: 100%;
}
</style>
