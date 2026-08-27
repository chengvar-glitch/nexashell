<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  isMacOSBrowser,
  isWindowsBrowser,
} from '@/core/utils/platform/platform-detection';
import AppTabs from '@/components/layout/AppTabs.vue';
import { Sun, Moon, Settings } from 'lucide-vue-next';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';
import { themeManager, THEME_CHANGED_EVENT } from '@/core/utils/theme-manager';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('WINDOW_TITLE_BAR');
const { t } = useI18n({ useScope: 'global' });

/**
 * WindowTitleBar Component
 *
 * Provides a custom title bar with platform-specific window controls (macOS
 * traffic lights, Windows control buttons). The tab strip (AppTabs) lives in
 * the same row — the whole bar is one line.
 */

const appWindow = getCurrentWindow();

// --- Reactive State ---
const showWindowControls = ref(false);
const isMacOS_OS = ref(false);
const isWindowsOS = ref(false);
const isFullscreen = ref(false);
const isMaximized = ref(false);

// --- Theme toggle + settings shortcuts ---
const isDarkTheme = ref(themeManager.getActualTheme() === 'dark');

/** Toggle between explicit light/dark, exiting 'auto' on first click. */
const toggleTheme = () => {
  themeManager.setTheme(isDarkTheme.value ? 'light' : 'dark');
};

const handleThemeChanged = () => {
  isDarkTheme.value = themeManager.getActualTheme() === 'dark';
};

const openSettings = () => {
  eventBus.emit(APP_EVENTS.OPEN_SETTINGS);
};

onMounted(async () => {
  try {
    // Detect platform for layout adjustments
    const isMac = isMacOSBrowser();
    const isWin = isWindowsBrowser();
    isMacOS_OS.value = isMac;
    isWindowsOS.value = isWin;
    showWindowControls.value = isMac || isWin;

    isFullscreen.value = await appWindow.isFullscreen();
    isMaximized.value = await appWindow.isMaximized();

    // Listen for resize events to update fullscreen state
    const unlistenResize = await appWindow.onResized(async () => {
      isFullscreen.value = await appWindow.isFullscreen();
      isMaximized.value = await appWindow.isMaximized();
    });

    (window as unknown as { __unlistenResize?: unknown }).__unlistenResize =
      unlistenResize;
  } catch (error) {
    logger.error('Failed to detect platform:', error);
    const isMac = isMacOSBrowser();
    const isWin = isWindowsBrowser();
    isMacOS_OS.value = isMac;
    isWindowsOS.value = isWin;
    showWindowControls.value = isMac || isWin;
  }

  // Keep the theme toggle icon in sync with the actual light/dark mode
  window.addEventListener(THEME_CHANGED_EVENT, handleThemeChanged);
});

onUnmounted(() => {
  window.removeEventListener(THEME_CHANGED_EVENT, handleThemeChanged);

  const unlisten = (window as unknown as { __unlistenResize?: () => void })
    .__unlistenResize;
  if (unlisten) {
    unlisten();
  }
});

/**
 * Closes the application window.
 */
const handleClose = async () => {
  try {
    await appWindow.close();
  } catch (error) {
    logger.error('Failed to close window:', error);
  }
};

/**
 * Minimizes the application window.
 */
const handleMinimize = async () => {
  try {
    await appWindow.minimize();
  } catch (error) {
    logger.error('Failed to minimize window:', error);
  }
};

/**
 * Toggles window maximization or macOS fullscreen mode.
 */
const handleMaximize = async () => {
  try {
    if (isMacOS_OS.value) {
      const isFullscreen = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!isFullscreen);
    } else {
      await appWindow.toggleMaximize();
    }
  } catch (error) {
    logger.error('Failed to maximize window:', error);
  }
};
</script>

<template>
  <div
    class="window-title-bar border-bottom"
    :class="{
      'fullscreen-mode': isFullscreen && isMacOS_OS,
      'is-windows': isWindowsOS,
    }"
    data-tauri-drag-region
  >
    <div class="left-section" data-tauri-drag-region>
      <!-- Native macOS traffic lights will float over this area -->
    </div>

    <AppTabs />

    <div class="right-section" data-tauri-drag-region>
      <div class="title-bar-actions">
        <button
          class="title-bar-btn"
          :title="t('window.toggleTheme')"
          :aria-label="t('window.toggleTheme')"
          @click="toggleTheme"
        >
          <Sun v-if="isDarkTheme" :size="15" />
          <Moon v-else :size="15" />
        </button>
        <button
          class="title-bar-btn"
          :title="t('window.settings')"
          :aria-label="t('window.settings')"
          @click="openSettings"
        >
          <Settings :size="15" />
        </button>
      </div>

      <div
        v-if="showWindowControls && isWindowsOS"
        class="window-controls windows-controls"
      >
        <button
          class="windows-control-btn minimize-btn"
          :aria-label="t('window.minimize')"
          @click="handleMinimize"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M0,5 L10,5" stroke="currentColor" stroke-width="1" />
          </svg>
        </button>
        <button
          class="windows-control-btn maximize-btn"
          :aria-label="t('window.maximize')"
          @click="handleMaximize"
        >
          <svg v-if="!isMaximized" width="10" height="10" viewBox="0 0 10 10">
            <rect
              x="0.5"
              y="0.5"
              width="9"
              height="9"
              stroke="currentColor"
              stroke-width="1"
              fill="none"
            />
          </svg>
          <svg v-else width="10" height="10" viewBox="0 0 10 10">
            <rect
              x="0.5"
              y="2.5"
              width="7"
              height="7"
              stroke="currentColor"
              stroke-width="1"
              fill="none"
            />
            <path
              d="M2.5,2.5 L2.5,0.5 L9.5,0.5 L9.5,7.5 L7.5,7.5"
              stroke="currentColor"
              stroke-width="1"
              fill="none"
            />
          </svg>
        </button>
        <button
          class="windows-control-btn close-btn"
          :aria-label="t('window.close')"
          @click="handleClose"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path
              d="M0,0 L10,10 M10,0 L0,10"
              stroke="currentColor"
              stroke-width="1"
            />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.window-title-bar {
  height: 38px;
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: stretch;
  background-color: var(--color-bg-secondary);
  position: relative;
  top: 0;
  z-index: 100;
}

.window-title-bar.is-windows .right-section {
  align-items: stretch;
  padding-right: 0; /* Ensure buttons are flush to the right edge */
}

.window-title-bar.fullscreen-mode {
  /* Adjust layout in fullscreen mode, remove traffic light buttons spacing */
  /* Reserve space for native system title bar in fullscreen mode */
  padding-top: env(safe-area-inset-top, 0);
}

.left-section,
.right-section {
  display: flex;
  align-items: center;
  height: 100%;
}

.left-section {
  padding-left: 16px;
  min-width: 80px; /* Reserve space for native macOS traffic lights */
}

.window-title-bar.is-windows .left-section {
  min-width: 0;
  padding-left: 12px;
}

.macos-controls {
  display: flex;
  gap: 8px;
}

.right-section {
  justify-content: flex-end;
}

.window-title-bar.is-windows .right-section {
  align-items: stretch;
}

.window-controls {
  display: flex;
  gap: 8px;
}

.title-bar-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-right: 8px;
}

.window-title-bar.is-windows .title-bar-actions {
  margin-right: 4px;
}

.title-bar-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background-color 0.1s, color 0.1s;
}

.title-bar-btn:hover {
  background-color: rgba(128, 128, 128, 0.15);
  color: var(--color-text-primary);
}

.window-title-bar.is-windows .window-controls {
  gap: 0;
}

.windows-control-btn {
  width: 46px;
  height: 100%;
  border: none;
  background: transparent;
  cursor: default; /* Windows uses standard arrow cursor for these buttons */
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-primary);
  transition: background-color 0.1s;
}

.windows-control-btn:hover {
  background-color: rgba(128, 128, 128, 0.15);
}

.windows-control-btn:active {
  background-color: rgba(128, 128, 128, 0.25);
}

.windows-control-btn.close-btn:hover {
  background-color: #e81123 !important;
  color: white !important;
}

.windows-control-btn.close-btn:active {
  background-color: #f1707a !important;
  color: white !important;
}
</style>
