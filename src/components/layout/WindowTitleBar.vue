<script setup lang="ts">
import { ref, onMounted, nextTick, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  isMacOSBrowser,
  isWindowsBrowser,
} from '@/core/utils/platform/platform-detection';
import SearchBox from '@/components/search/SearchBox.vue';
import SearchDropdown from '@/components/search/SearchDropdown.vue';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';

/**
 * WindowTitleBar Component
 *
 * Provides a custom title bar with platform-specific window controls (macOS traffic lights,
 * Windows control buttons) and a centralized search functionality.
 */

const appWindow = getCurrentWindow();

// --- Reactive State ---
const showWindowControls = ref(false);
const isMacOS_OS = ref(false);
const isWindowsOS = ref(false);
const isFullscreen = ref(false);
const isMaximized = ref(false);
const isFocused = ref(true);

// --- Search Functionality Refs ---
const searchBoxRef = ref<InstanceType<typeof SearchBox> | null>(null);
const searchDropdownRef = ref<InstanceType<typeof SearchDropdown> | null>(null);
const showSearchDropdown = ref(false);
const searchBoxElement = ref<HTMLElement | undefined>(undefined);
const searchQuery = ref('');

/**
 * Handles search box focus to display the dropdown and update its position.
 */
const onSearchBoxFocus = () => {
  showSearchDropdown.value = true;
  nextTick(() => {
    if (searchBoxRef.value) {
      // Synchronize input element for dropdown anchoring
      const inputElement = searchBoxRef.value.$el.querySelector('input');
      if (inputElement) {
        searchBoxElement.value = inputElement;
      } else if (searchBoxRef.value.$el) {
        searchBoxElement.value = searchBoxRef.value.$el;
      }
      if (searchDropdownRef.value) {
        (searchDropdownRef.value as any).updatePosition?.();
      }
    }
  });
};

/**
 * Handles search box blur with a delay to allow interaction with dropdown items.
 */
const onSearchBoxBlur = () => {
  // Delay closing to allow user to click dropdown options
  setTimeout(() => {
    const activeElement = document.activeElement;
    const dropdownElement = document.querySelector('.search-dropdown');
    // Prevent closing if focus moved into the dropdown itself
    if (dropdownElement && dropdownElement.contains(activeElement)) {
      return;
    }
    showSearchDropdown.value = false;
  }, 150);
};

/**
 * Forwards KeyboardEvent to the SearchDropdown component.
 */
const onSearchBoxKeyDown = (event: KeyboardEvent) => {
  if (!showSearchDropdown.value || !searchDropdownRef.value) {
    return;
  }
  (searchDropdownRef.value as any).handleKeyDown(event);
};

/**
 * Opens the dropdown when the user starts typing.
 */
const onSearchBoxInput = () => {
  if (!showSearchDropdown.value) {
    showSearchDropdown.value = true;
  }
};

/**
 * Forwards KeyUp events to the SearchDropdown component.
 */
const onSearchBoxKeyUp = (event: KeyboardEvent) => {
  if (!showSearchDropdown.value || !searchDropdownRef.value) {
    return;
  }
  (searchDropdownRef.value as any).handleKeyUp(event);
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

    const unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }) => {
      isFocused.value = focused;
    });

    (window as any).__unlistenResize = unlistenResize;
    (window as any).__unlistenFocus = unlistenFocus;
  } catch (error) {
    console.error('Failed to detect platform:', error);
    const isMac = isMacOSBrowser();
    const isWin = isWindowsBrowser();
    isMacOS_OS.value = isMac;
    isWindowsOS.value = isWin;
    showWindowControls.value = isMac || isWin;
  }

  // Listen for global search focus events
  eventBus.on(APP_EVENTS.FOCUS_SEARCH, handleFocusSearch);
});

onUnmounted(() => {
  eventBus.off(APP_EVENTS.FOCUS_SEARCH, handleFocusSearch);

  if ((window as any).__unlistenResize) {
    (window as any).__unlistenResize();
  }
  if ((window as any).__unlistenFocus) {
    (window as any).__unlistenFocus();
  }
});

/**
 * Programmatically focuses the search box.
 */
const handleFocusSearch = () => {
  nextTick(() => {
    if (searchBoxRef.value) {
      searchBoxRef.value.focus();
      showSearchDropdown.value = true;
    }
  });
};

/**
 * Closes the application window.
 */
const handleClose = async () => {
  try {
    await appWindow.close();
  } catch (error) {
    console.error('Failed to close window:', error);
  }
};

/**
 * Minimizes the application window.
 */
const handleMinimize = async () => {
  try {
    await appWindow.minimize();
  } catch (error) {
    console.error('Failed to minimize window:', error);
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
    console.error('Failed to maximize window:', error);
  }
};
</script>

<template>
  <div
    class="window-title-bar glass-medium border-bottom"
    :class="{ 'fullscreen-mode': isFullscreen && isMacOS_OS, 'is-windows': isWindowsOS }"
    data-tauri-drag-region
  >
    <div class="left-section" data-tauri-drag-region>
      <!-- Window controls for macOS -->
      <div
        v-if="showWindowControls && isMacOS_OS && !isFullscreen"
        class="window-controls macos-controls"
        :class="{ 'window-inactive': !isFocused }"
      >
        <button
          class="window-control-btn close-btn"
          aria-label="Close"
          @click="handleClose"
        >
          <svg viewBox="0 0 12 12" class="macos-icon">
            <path
              fill="none"
              stroke="currentColor"
              stroke-width="1.1"
              d="M3.5,3.5 L8.5,8.5 M8.5,3.5 L3.5,8.5"
            />
          </svg>
        </button>
        <button
          class="window-control-btn minimize-btn"
          aria-label="Minimize"
          @click="handleMinimize"
        >
          <svg viewBox="0 0 12 12" class="macos-icon">
            <path
              fill="none"
              stroke="currentColor"
              stroke-width="1.1"
              d="M2.5,6 L9.5,6"
            />
          </svg>
        </button>
        <button
          class="window-control-btn maximize-btn"
          aria-label="Fullscreen"
          @click="handleMaximize"
        >
          <svg viewBox="0 0 12 12" class="macos-icon">
            <path
              fill="currentColor"
              d="M4.5,3.5 L3.5,3.5 L3.5,4.5 L4.5,3.5 Z M7.5,8.5 L8.5,8.5 L8.5,7.5 L7.5,8.5 Z M3.5,7.5 L3.5,8.5 L4.5,8.5 L3.5,7.5 Z M8.5,4.5 L8.5,3.5 L7.5,3.5 L8.5,4.5 Z"
            />
          </svg>
        </button>
      </div>
    </div>

    <div class="center-section" data-tauri-drag-region>
      <SearchBox
        ref="searchBoxRef"
        v-model="searchQuery"
        class="disable-selection"
        @focus="onSearchBoxFocus"
        @blur="onSearchBoxBlur"
        @keydown="onSearchBoxKeyDown"
        @keyup="onSearchBoxKeyUp"
        @input="onSearchBoxInput"
      />
    </div>

    <div class="right-section" data-tauri-drag-region>
      <div
        v-if="showWindowControls && isWindowsOS"
        class="window-controls windows-controls"
      >
        <button
          class="windows-control-btn minimize-btn"
          aria-label="Minimize"
          @click="handleMinimize"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M0,5 L10,5" stroke="currentColor" stroke-width="1" />
          </svg>
        </button>
        <button
          class="windows-control-btn maximize-btn"
          aria-label="Maximize"
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
          aria-label="Close"
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

    <SearchDropdown
      ref="searchDropdownRef"
      v-model:visible="showSearchDropdown"
      :anchor-element="searchBoxElement"
      :search-query="searchQuery"
    />
  </div>
</template>

<style scoped>
.window-title-bar {
  height: 38px;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: stretch;
  background-color: var(--color-bg-primary);
  position: relative;
  top: 0;
  z-index: 100;
  border-radius: var(--radius-2xl) var(--radius-2xl) 0 0;
}

.window-title-bar.is-windows {
  border-radius: 0;
}

.window-title-bar.fullscreen-mode {
  /* Adjust layout in fullscreen mode, remove traffic light buttons spacing */
  border-radius: 0;
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

.center-section {
  display: flex;
  justify-content: center;
  align-items: center;
}

.macos-controls {
  display: flex;
  gap: 8px;
  cursor: default;
}

.window-control-btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 0.5px solid rgba(0, 0, 0, 0.12);
  cursor: default;
  position: relative;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s ease;
  box-shadow: inset 0 0 1px rgba(255, 255, 255, 0.1);
}

.macos-icon {
  width: 100%; /* Icons use 12x12 viewBox, so fill container */
  height: 100%;
  opacity: 0;
  transition: opacity 0.15s ease;
  pointer-events: none;
}

.macos-controls:hover .macos-icon {
  opacity: 1;
}

.close-btn {
  background: #ff5f56;
}

.close-btn:active {
  background: #bf4942;
}

.close-btn .macos-icon {
  color: #4b0002;
}

.minimize-btn {
  background: #ffbd2e;
}

.minimize-btn:active {
  background: #be8e22;
}

.minimize-btn .macos-icon {
  color: #975700;
}

.maximize-btn {
  background: #27c93f;
}

.maximize-btn:active {
  background: #1e9630;
}

.maximize-btn .macos-icon {
  color: #006500;
}

.window-inactive .window-control-btn {
  background: #e1e1e1 !important;
  border-color: #cfcfcf !important;
  box-shadow: none !important;
}

.window-inactive .macos-icon {
  display: none;
}

.windows-control-btn {
  width: 46px;
  height: 100%;
  border: none;
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-primary);
  transition: background-color 0.1s; /* Windows has a very fast fade */
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

.disable-selection {
  user-select: none;
  -webkit-user-select: none;
}
</style>
