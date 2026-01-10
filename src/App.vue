<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, provide } from 'vue';
import WindowTitleBar from './components/layout/WindowTitleBar.vue';
import AppTabs from './components/layout/AppTabs.vue';
import AppContent from './components/layout/AppContent.vue';
import { shortcutManager, PredefinedShortcuts } from './utils/shortcut-manager';
import { themeManager } from './utils/theme-manager';

const showSettings = ref(false);
provide('showSettings', showSettings);

onMounted(() => {
  // Initialize theme system
  themeManager.initialize();

  shortcutManager.register(PredefinedShortcuts.QUIT_APP);
  shortcutManager.register(PredefinedShortcuts.CLOSE_WINDOW);
  shortcutManager.register(PredefinedShortcuts.OPEN_SETTINGS);
  shortcutManager.register(PredefinedShortcuts.NEW_LOCAL_TAB);
  shortcutManager.register(PredefinedShortcuts.NEW_SSH_TAB);
  shortcutManager.register(PredefinedShortcuts.CLOSE_CURRENT_TAB);
  shortcutManager.register(PredefinedShortcuts.FOCUS_SEARCH);
  shortcutManager.register(PredefinedShortcuts.CLOSE_DIALOG);

  window.addEventListener('app:open-settings', () => {
    showSettings.value = true;
  });

  window.addEventListener('app:close-dialog', () => {
    showSettings.value = false;
  });
});

onBeforeUnmount(() => {
  shortcutManager.unregisterAll();
});
</script>

<template>
  <div id="app" class="app-wrapper">
    <div class="app-root">
      <WindowTitleBar />
      <AppTabs />
      <AppContent />
    </div>
  </div>
</template>

<style scoped>
.app-wrapper {
  width: 100vw;
  height: 100vh;
  padding: 0;
  background: transparent;
  display: flex;
  align-items: center;
  justify-content: center;
}

.app-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  border-radius: var(--radius-2xl);
  overflow: hidden;
  background-color: transparent;
  box-shadow:
    0 0 0 0.5px rgba(0, 0, 0, 0.1),
    var(--shadow-2xl);
  border: none;
  transition: all var(--transition-base);
}

/* Fullscreen mode: remove rounded corners and borders */
@media (display-mode: fullscreen) {
  .app-root {
    border-radius: 0;
    border: none;
  }
}

/* Dark theme optimization */
@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .app-root {
    box-shadow:
      0 0 0 0.5px rgba(255, 255, 255, 0.1),
      var(--shadow-2xl);
  }
}

:root.theme-dark .app-root {
  box-shadow:
    0 0 0 0.5px rgba(255, 255, 255, 0.1),
    var(--shadow-2xl);
}
</style>
<style>
:root {
  /* macOS system fonts priority */
  font-family:
    -apple-system,
    BlinkMacSystemFont,
    /* macOS SF Pro */ 'SF Pro Text',
    'SF Pro Display',
    /* Windows Segoe UI */ 'Segoe UI',
    /* Generic sans-serif */ system-ui,
    /* macOS Chinese fonts */ 'PingFang SC',
    'Hiragino Sans GB',
    /* Windows Chinese fonts */ 'Microsoft YaHei UI',
    'Microsoft YaHei',
    /* Linux Chinese fonts */ 'WenQuanYi Micro Hei',
    /* Fallback fonts */ 'Helvetica Neue',
    'Helvetica',
    'Arial',
    sans-serif;
  font-size: 14px;
  line-height: 1.6;
  font-weight: 400;

  color: #0f0f0f;
  background-color: transparent;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body,
#app {
  overflow: hidden;
  background: transparent !important;
  margin: 0 !important;
  padding: 0 !important;
  width: 100%;
  height: 100%;
  border: none !important;
  outline: none !important;
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) {
    color: #f6f6f6;
    background-color: transparent;
  }
}

:root.theme-dark {
  color: #f6f6f6;
  background-color: transparent;
}

/* Disable text selection across the entire app */
#app {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
}

div[role='region'] {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
}
</style>
