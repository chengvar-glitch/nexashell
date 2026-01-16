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

    // Listen for resize events to update fullscreen state
    const unlistenResize = await appWindow.onResized(async () => {
      isFullscreen.value = await appWindow.isFullscreen();
    });

    (window as any).__unlistenResize = unlistenResize;
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
    :class="{ 'fullscreen-mode': isFullscreen && isMacOS_OS }"
    data-tauri-drag-region
  >
    <div
      class="left-section"
      data-tauri-drag-region
    >
      <!-- Window controls for macOS -->
      <div
        v-if="showWindowControls && isMacOS_OS && !isFullscreen"
        class="window-controls macos-controls"
      >
        <button
          class="window-control-btn close-btn"
          aria-label="Close"
          @click="handleClose"
        />
        <button
          class="window-control-btn minimize-btn"
          aria-label="Minimize"
          @click="handleMinimize"
        />
        <button
          class="window-control-btn maximize-btn"
          aria-label="Fullscreen"
          @click="handleMaximize"
        />
      </div>
    </div>

    <div
      class="center-section"
      data-tauri-drag-region
    >
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

    <div
      class="right-section"
      data-tauri-drag-region
    >
      <div
        v-if="showWindowControls && isWindowsOS"
        class="window-controls windows-controls"
      >
        <button
          class="windows-control-btn minimize-btn"
          aria-label="Minimize"
          @click="handleMinimize"
        >
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
          >
            <path
              d="M0,5 L10,5"
              stroke="currentColor"
              stroke-width="1"
            />
          </svg>
        </button>
        <button
          class="windows-control-btn maximize-btn"
          aria-label="Maximize"
          @click="handleMaximize"
        >
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
          >
            <rect
              x="0"
              y="0"
              width="10"
              height="10"
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
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
          >
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
  align-items: center;
  background-color: var(--color-bg-primary);
  position: relative;
  top: 0;
  z-index: 100;
  border-radius: var(--radius-2xl) var(--radius-2xl) 0 0;
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
  padding-left: 12px;
}

.right-section {
  justify-content: flex-end;
}

.center-section {
  display: flex;
  justify-content: center;
  align-items: center;
}

.window-controls {
  display: flex;
  gap: 8px;
}

.windows-controls {
  gap: 0;
}

.window-control-btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  transition: all var(--transition-base);
  position: relative;
}

.window-control-btn:hover {
  filter: brightness(0.9);
}

.close-btn {
  background-color: var(--color-macos-close);
}

.minimize-btn {
  background-color: var(--color-macos-minimize);
}

.maximize-btn {
  background-color: var(--color-macos-maximize);
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
  transition: background-color var(--transition-base);
}

.windows-control-btn:hover {
  background-color: var(--color-bg-hover);
}

.windows-control-btn.close-btn:hover {
  background-color: #e81123;
  color: white;
}

.disable-selection {
  user-select: none;
  -webkit-user-select: none;
}
</style>
