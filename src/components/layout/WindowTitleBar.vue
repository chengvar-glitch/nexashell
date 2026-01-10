<script setup lang="ts">
import { ref, onMounted, inject, Ref, nextTick, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isMacOS, isWindows } from '../../utils/tauri-system';
import SearchBox from '../ui/SearchBox.vue';
import SettingsPanel from '../ui/SettingsPanel.vue';
import ShortcutHint from '../ui/ShortcutHint.vue';
import SearchDropdown from '../ui/SearchDropdown.vue';
import { MoreHorizontal } from 'lucide-vue-next';

const appWindow = getCurrentWindow();
const showWindowControls = ref(false);
const isWindowsOS = ref(false);
const isMacOS_OS = ref(false);
const isFullscreen = ref(false);

const searchBoxRef = ref<InstanceType<typeof SearchBox> | null>(null);
const searchDropdownRef = ref<InstanceType<typeof SearchDropdown> | null>(null);
const showSearchDropdown = ref(false);
const searchBoxElement = ref<HTMLElement | null>(null);

const showSettings = inject<Ref<boolean>>('showSettings', ref(false));

const onSearchBoxFocus = () => {
  showSearchDropdown.value = true;
  nextTick(() => {
    if (searchBoxRef.value) {
      const inputElement = searchBoxRef.value.$el.querySelector('input');
      searchBoxElement.value = inputElement || searchBoxRef.value.$el;
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

const onSearchBoxKeyUp = (event: KeyboardEvent) => {
  if (!showSearchDropdown.value || !searchDropdownRef.value) {
    return;
  }
  (searchDropdownRef.value as any).handleKeyUp(event);
};

onMounted(async () => {
  try {
    const [isMac, isWin] = await Promise.all([isMacOS(), isWindows()]);
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
    const isMac = navigator.userAgent.includes('Mac');
    const isWin = navigator.userAgent.includes('Windows');
    isMacOS_OS.value = isMac;
    isWindowsOS.value = isWin;
    showWindowControls.value = isMac || isWin;
  }
  
  window.addEventListener('app:focus-search', handleFocusSearch);
});

onUnmounted(() => {
  window.removeEventListener('app:focus-search', handleFocusSearch);
  
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
  <div class="window-title-bar glass-medium border-bottom" :class="{ 'fullscreen-mode': isFullscreen && isMacOS_OS }" data-tauri-drag-region>
    <!-- Window controls for macOS -->
    <div v-if="showWindowControls && isMacOS_OS && !isFullscreen" class="window-controls macos-controls">
      <button class="window-control-btn close-btn" @click="handleClose" aria-label="Close"></button>
      <button class="window-control-btn minimize-btn" @click="handleMinimize" aria-label="Minimize"></button>
      <button class="window-control-btn maximize-btn" @click="handleMaximize" aria-label="Fullscreen"></button>
    </div>
    
    <SearchBox 
      ref="searchBoxRef" 
      class="disable-selection"
      @focus="onSearchBoxFocus" 
      @blur="onSearchBoxBlur"
      @keydown="onSearchBoxKeyDown"
      @keyup="onSearchBoxKeyUp"
    />
    
    <SearchDropdown 
      ref="searchDropdownRef" 
      v-model:visible="showSearchDropdown"
      :anchor-element="searchBoxElement"
    />
    
    <div class="settings-container">
      <ShortcutHint text="Cmd+," position="bottom">
        <button class="btn-icon" @click="handleOpenSettings">
          <MoreHorizontal :size="16" />
        </button>
      </ShortcutHint>
    </div>
    
    <div v-if="showWindowControls && isWindowsOS" class="window-controls windows-controls">
      <button class="windows-control-btn minimize-btn" @click="handleMinimize" aria-label="Minimize">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <path d="M0,5 L10,5" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
      <button class="windows-control-btn maximize-btn" @click="handleMaximize" aria-label="Maximize">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <rect x="0" y="0" width="10" height="10" stroke="currentColor" stroke-width="1" fill="none" />
        </svg>
      </button>
      <button class="windows-control-btn close-btn" @click="handleClose" aria-label="Close">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <path d="M0,0 L10,10 M10,0 L0,10" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
    </div>
  </div>
  
  <SettingsPanel v-model:visible="showSettings" />
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

.windows-control-btn {
  width: 46px;
  height: 40px;
  border: none;
  background-color: transparent;
  color: var(--color-text-primary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--transition-base);
}

.windows-control-btn:hover {
  background-color: var(--color-interactive-hover);
}

.windows-control-btn.close-btn:hover {
  background-color: #e81123;
  color: #fff;
}

.settings-container {
  display: flex;
  justify-content: flex-end;
  padding: 0 16px;
}

.settings-btn {
  padding: 5px;
  border-radius: var(--radius-md);
  border: none;
  background-color: transparent;
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all var(--transition-base);
  display: flex;
  align-items: center;
  justify-content: center;
}

.settings-btn:hover {
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.disable-selection {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .close-btn {
    background-color: var(--color-macos-close);
  }
  
  :root:not(.theme-light) .close-btn:hover {
    background-color: var(--color-macos-close-hover);
  }
}

:root.theme-dark .close-btn {
  background-color: var(--color-macos-close);
}

:root.theme-dark .close-btn:hover {
  background-color: var(--color-macos-close-hover);
}
</style>