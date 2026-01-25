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

const logger = createLogger('REMOTE_CONNECTION_VIEW');

const sessionStore = useSessionStore();
const settingsStore = useSettingsStore();

// Terminal configuration constants - Now acting as defaults or base
const TERMINAL_CONFIG = {
  THEME: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    selectionBackground: '#facc15', // Bright yellow background for selection
    selectionForeground: '#000000', // Black text for selected content
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
  tabType?: string;
  ip?: string;
  port?: number;
  username?: string;
  password?: string;
}

const props = withDefaults(defineProps<Props>(), {
  sessionId: '',
  tabType: 'ssh',
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
 * Establish connection via session store and API
 */
const connectSession = async (cols: number, rows: number): Promise<void> => {
  if (!props.sessionId) {
    throw new Error('sessionId is required');
  }

  if (sessionStore.hasSession(props.sessionId)) {
    return;
  }

  try {
    if (props.tabType === 'terminal') {
      logger.info('Creating local terminal session', {
        sessionId: props.sessionId,
      });
      await sessionStore.createLocalSession(
        props.sessionId,
        props.sessionId,
        cols,
        rows
      );
    } else {
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
    }

    // Retrieve buffered initial output (e.g., welcome banner, login prompts)
    // Wait for the backend to complete the initial buffering phase.
    // A delay of ~2 seconds ensures the full initial sequence is captured for SSH.
    if (props.tabType !== 'terminal') {
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
    }
  } catch (error) {
    logger.error('Connection failed', error);
    throw error;
  }
};

/**
 * Disconnect session
 */
const disconnectSession = async (): Promise<void> => {
  if (!props.sessionId) return;

  try {
    // Session store handles the specific disconnect logic based on session type
    await sessionStore.disconnectSession(props.sessionId);
  } catch (error) {
    logger.error('Disconnect failed', error);
  }
};

/**
 * Cleanup terminal resources
 */
const cleanupResources = async (): Promise<void> => {
  if (props.sessionId) {
    await disconnectSession();
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

  // Handle keyboard shortcuts and allow global app shortcuts to bubble up
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    if (event.type !== 'keydown') return true;

    const isMac = navigator.userAgent.includes('Mac');
    const isControlKey = isMac ? event.metaKey : event.ctrlKey;
    const key = event.key.toLowerCase();

    // 1. Allow global app shortcuts to bubble up to the window
    // This includes Cmd+T, Cmd+Shift+T, Cmd+W, Cmd+K, Cmd+, and Cmd+Shift+P
    if (
      isControlKey &&
      (['t', 'w', 'k', ','].includes(key) || event.shiftKey)
    ) {
      return false;
    }

    // 2. Handle clipboard shortcuts (Cmd+C/V on Mac, Ctrl+C/V on Windows/Linux)
    if (isControlKey && event.code === 'KeyC') {
      if (terminal?.hasSelection()) {
        const selection = terminal.getSelection();
        if (selection) {
          navigator.clipboard.writeText(selection);
        }
        return false;
      }
      return isMac ? false : true;
    }

    if (isControlKey && event.code === 'KeyV') {
      navigator.clipboard.readText().then(text => {
        if (text && props.sessionId) {
          emit(`ssh-input-${props.sessionId}`, { input: text });
        }
      });
      return false;
    }

    // 3. Other internal shortcuts
    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyA') {
      terminal?.selectAll();
      return false;
    }

    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyF') {
      toggleSearch();
      return false;
    }

    if ((event.metaKey || event.ctrlKey) && event.code === 'KeyL') {
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

  // Handle activation when switching back to this tab (KeepAlive support)
  onActivated(() => {
    nextTick(() => {
      if (fitAddon) {
        fitAddon.fit();
        terminal?.focus();

        // Re-sync terminal dimensions with the backend after activation
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

            await connectSession(terminal.cols, terminal.rows);

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
          logger.error('Connection failed', error);
        }
      }
    },
    { immediate: true }
  );

  /**
   * Handle terminal input
   */
  // Key sequence mapping for terminal input
  /**
   * Terminal input handling
   * Using onData instead of onKey to properly handle IME (Chinese input)
   */
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

  terminal.onData(async (data: string) => {
    const hasSession =
      props.sessionId && sessionStore.hasSession(props.sessionId);

    if (hasSession) {
      inputBuffer += data;

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
    } else if (data.length === 1 && data >= ' ') {
      // No active session: echo printable characters locally
      terminal?.write(data);
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
  <div class="remote-connection-view">
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
.remote-connection-view {
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
