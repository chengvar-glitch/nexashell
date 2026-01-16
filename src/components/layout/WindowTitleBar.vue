<script setup lang="ts">
import { ref, onMounted, inject, Ref, nextTick, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  isMacOSBrowser,
  isWindowsBrowser,
} from '@/core/utils/platform/platform-detection';
import SearchBox from '@/components/search/SearchBox.vue';
import ShortcutHint from '@/components/common/ShortcutHint.vue';
import SearchDropdown from '@/components/search/SearchDropdown.vue';
import { MoreHorizontal } from 'lucide-vue-next';
import { SHOW_SETTINGS_KEY } from '@/core/types';
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';

const appWindow = getCurrentWindow();
const showWindowControls = ref(false);
const isMacOS_OS = ref(false);
const isWindowsOS = ref(false);
const isFullscreen = ref(false);

const searchBoxRef = ref<InstanceType<typeof SearchBox> | null>(null);
const searchDropdownRef = ref<InstanceType<typeof SearchDropdown> | null>(null);
const showSearchDropdown = ref(false);
const searchBoxElement = ref<HTMLElement | undefined>(undefined);
const searchQuery = ref('');

const showSettings = inject<Ref<boolean>>(SHOW_SETTINGS_KEY, ref(false));

const onSearchBoxFocus = () => {
  showSearchDropdown.value = true;
  nextTick(() => {
    if (searchBoxRef.value) {
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

const onSearchBoxBlur = () => {
  // Delay closing to allow user to click dropdown options
  setTimeout(() => {
    // Check if focus is within dropdown menu
    const activeElement = document.activeElement;
    const dropdownElement = document.querySelector('.search-dropdown');
    if (dropdownElement && dropdownElement.contains(activeElement)) {
      return;
    }
    showSearchDropdown.value = false;
  }, 150);
};

const onSearchBoxKeyDown = (event: KeyboardEvent) => {
  if (!showSearchDropdown.value || !searchDropdownRef.value) {
    return;
  }
  (searchDropdownRef.value as any).handleKeyDown(event);
};

const onSearchBoxInput = () => {
  // Show dropdown menu if it's currently hidden
  if (!showSearchDropdown.value) {
    showSearchDropdown.value = true;
  }
};

const onSearchBoxKeyUp = (event: KeyboardEvent) => {
  if (!showSearchDropdown.value || !searchDropdownRef.value) {
    return;
  }
  (searchDropdownRef.value as any).handleKeyUp(event);
};

onMounted(async () => {
  try {
    const isMac = isMacOSBrowser();
    const isWin = isWindowsBrowser();
    isMacOS_OS.value = isMac;
    isWindowsOS.value = isWin;
    showWindowControls.value = isMac || isWin;

    isFullscreen.value = await appWindow.isFullscreen();

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

  eventBus.on(APP_EVENTS.FOCUS_SEARCH, handleFocusSearch);
});

onUnmounted(() => {
  eventBus.off(APP_EVENTS.FOCUS_SEARCH, handleFocusSearch);

  if ((window as any).__unlistenResize) {
    (window as any).__unlistenResize();
  }
});

const handleFocusSearch = () => {
  nextTick(() => {
    if (searchBoxRef.value) {
      searchBoxRef.value.focus();
      showSearchDropdown.value = true;
    }
  });
};

const handleClose = async () => {
  try {
    await appWindow.close();
  } catch (error) {
    console.error('Failed to close window:', error);
  }
};

const handleMinimize = async () => {
  try {
    await appWindow.minimize();
  } catch (error) {
    console.error('Failed to minimize window:', error);
  }
};

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

const handleOpenSettings = () => {
  showSettings.value = true;
};
</script>

<template>
  <div
    class="window-title-bar glass-medium border-bottom"
    :class="{ 'fullscreen-mode': isFullscreen && isMacOS_OS }"
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

    <SearchDropdown
      ref="searchDropdownRef"
      v-model:visible="showSearchDropdown"
      :anchor-element="searchBoxElement"
      :search-query="searchQuery"
    />

    <div class="settings-container">
      <ShortcutHint text="Cmd+," position="bottom">
        <button class="btn-icon" @click="handleOpenSettings">
          <MoreHorizontal :size="16" />
        </button>
      </ShortcutHint>
    </div>

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
        <svg width="10" height="10" viewBox="0 0 10 10">
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
</template>

<style scoped>
.window-title-bar {
  height: 40px;
  display: grid;
  grid-template-columns: auto 1fr auto auto;
  align-items: center;
  padding: 0 0 0 12px;
  background-color: var(--color-bg-primary);
  position: relative;
  top: 0;
  z-index: 100;
  border-radius: var(--radius-2xl) var(--radius-2xl) 0 0;
}

.window-title-bar.fullscreen-mode {
  /* Adjust layout in fullscreen mode, remove traffic light buttons spacing */
  grid-template-columns: 1fr auto auto;
  padding-left: 16px;
  border-radius: 0;
  /* Reserve space for native system title bar in fullscreen mode */
  padding-top: env(safe-area-inset-top, 0);
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

.settings-container {
  display: flex;
  align-items: center;
  padding-right: 8px;
}

.btn-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all var(--transition-base);
}

.btn-icon:hover {
  background-color: var(--color-bg-hover);
  color: var(--color-text-primary);
}

.windows-control-btn {
  width: 46px;
  height: 32px;
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
