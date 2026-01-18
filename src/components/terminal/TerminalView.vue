<script setup lang="ts">
import { onMounted, onUnmounted, ref, nextTick, watch, onActivated } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { SearchAddon } from '@xterm/addon-search';
import '@xterm/xterm/css/xterm.css';
import { useSessionStore } from '@/features/session';
import { sessionApi } from '@/features/session';
import { createLogger } from '@/core/utils/logger';
import { listen, UnlistenFn, emit } from '@tauri-apps/api/event';
import { useSettingsStore } from '@/features/settings';

const logger = createLogger('TERMINAL_VIEW');

const sessionStore = useSessionStore();
const settingsStore = useSettingsStore();

// Terminal configuration constants - Now acting as defaults or base
const TERMINAL_CONFIG = {
  THEME: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    selectionBackground: '#facc15', // 醒目的黄色背景
    selectionForeground: '#000000', // 黑色文字
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

const terminalRef = ref<HTMLElement>();
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;

// Search state
const showSearch = ref(false);
const searchQuery = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

/**
 * Search functionality
 */
const toggleSearch = () => {
  showSearch.value = !showSearch.value;
  if (showSearch.value) {
    nextTick(() => {
      searchInputRef.value?.focus();
    });
  } else {
    terminal?.focus();
  }
};

const handleSearch = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value, { incremental: true });
  }
};

const handleSearchNext = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value);
  }
};

const handleSearchPrev = () => {
  if (searchAddon && searchQuery.value) {
    searchAddon.findPrevious(searchQuery.value);
  }
};

const closeSearch = () => {
  showSearch.value = false;
  searchQuery.value = '';
  nextTick(() => {
    terminal?.focus();
  });
};

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
    scrollback: settingsStore.terminal.scrollback,
    fontSize: settingsStore.terminal.fontSize,
    fontFamily: settingsStore.terminal.fontFamily,
    rows: 24,
    cols: 80,
    cursorBlink: settingsStore.terminal.cursorBlink,
    cursorStyle: settingsStore.terminal.cursorStyle,
    theme: TERMINAL_CONFIG.THEME,
  });

  // Watch for cursor style changes
  watch(
    () => settingsStore.terminal.cursorStyle,
    newStyle => {
      if (terminal) {
        terminal.options.cursorStyle = newStyle;
      }
    }
  );

  // Watch for cursor blink changes
  watch(
    () => settingsStore.terminal.cursorBlink,
    newBlink => {
      if (terminal) {
        terminal.options.cursorBlink = newBlink;
      }
    }
  );

  // Watch for font size changes
  watch(
    () => settingsStore.terminal.fontSize,
    newSize => {
      if (terminal) {
        terminal.options.fontSize = newSize;
        fitAddon?.fit();
      }
    }
  );

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  searchAddon = new SearchAddon();
  terminal.loadAddon(searchAddon);

  // Use WebGL renderer for better performance
  try {
    terminal.loadAddon(new WebglAddon());
  } catch {
    logger.warn('WebGL addon unavailable, using standard renderer');
  }

  terminal.open(terminalRef.value);

  // Handle clipboard shortcuts (Cmd+C/V on Mac, Ctrl+C/V on Windows/Linux)
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    if (event.type !== 'keydown') return true;

    const isMac = navigator.userAgent.includes('Mac');
    const isControlKey = isMac ? event.metaKey : event.ctrlKey;

    // Copy: Cmd+C (Mac) or Ctrl+C (Win/Linux)
    if (isControlKey && event.code === 'KeyC') {
      if (terminal?.hasSelection()) {
        const selection = terminal.getSelection();
        if (selection) {
          navigator.clipboard.writeText(selection);
        }
        return false;
      }
      // If no selection on Windows, return true to let it be handled as SIGINT (Ctrl+C)
      return isMac ? false : true;
    }

    // Paste: Cmd+V (Mac) or Ctrl+V (Win/Linux)
    if (isControlKey && event.code === 'KeyV') {
      navigator.clipboard.readText().then(text => {
        if (text && props.sessionId) {
          // Send pasted text to the backend session
          emit(`ssh-input-${props.sessionId}`, { input: text });
        }
      });
      return false;
    }

    // Select All: Cmd+A or Ctrl+A
    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyA') {
      terminal?.selectAll();
      return false;
    }

    // Search: Cmd+F or Ctrl+F
    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyF') {
      toggleSearch();
      return false;
    }

    // Clear: Cmd+Shift+K or Ctrl+Shift+K
    if (
      (event.metaKey || event.ctrlKey) &&
      event.shiftKey &&
      event.code === 'KeyK'
    ) {
      terminal?.clear();
      return false;
    }

    return true;
  });

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
      // Check if this is a global shortcut that should bubble up
      const isMac = navigator.userAgent.includes('Mac');
      const isShortcut =
        (isMac ? domEvent.metaKey : domEvent.ctrlKey) &&
        ['w', 'k', 't', ','].includes(key.toLowerCase());

      if (!isShortcut) {
        domEvent.preventDefault();
        domEvent.stopPropagation();
      }

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
    <div v-if="showSearch" class="terminal-search-box">
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        type="text"
        placeholder="Search..."
        @input="handleSearch"
        @keydown.enter.exact.stop.prevent="handleSearchNext"
        @keydown.shift.enter.stop.prevent="handleSearchPrev"
        @keydown.escape.stop.prevent="closeSearch"
      />
      <div class="search-actions">
        <button @click.stop="handleSearchPrev">↑</button>
        <button @click.stop="handleSearchNext">↓</button>
        <button @click.stop="closeSearch">×</button>
      </div>
    </div>
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
  position: relative;
}

.terminal-search-box {
  position: absolute;
  top: 10px;
  right: 20px;
  z-index: 100;
  display: flex;
  align-items: center;
  background-color: #2d2d2d;
  padding: 4px 10px;
  border-radius: 4px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  border: 1px solid #444;
  transition: border-color 0.2s;
}

.terminal-search-box:focus-within {
  border-color: #facc15;
}

.terminal-search-box input {
  background: transparent;
  border: none;
  color: #fff;
  outline: none;
  font-size: 13px;
  padding: 4px;
  width: 200px;
}

.search-actions {
  display: flex;
  gap: 4px;
  margin-left: 8px;
}

.search-actions button {
  background: transparent;
  border: none;
  color: #888;
  cursor: pointer;
  padding: 4px 6px;
  font-size: 14px;
  line-height: 1;
  border-radius: 3px;
  transition: all 0.2s;
}

.search-actions button:hover {
  color: #fff;
  background-color: #444;
}

.terminal-container {
  width: 100%;
  height: 100%;
}
</style>
