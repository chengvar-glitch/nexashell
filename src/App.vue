<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, provide } from 'vue';
import WindowTitleBar from '@/components/layout/WindowTitleBar.vue';
import AppTabs from '@/components/layout/AppTabs.vue';
import AppContent from '@/components/layout/AppContent.vue';
import SSHConnectionForm from '@/components/ssh/SSHConnectionForm.vue';
import { shortcutManager, PredefinedShortcuts } from './utils/shortcut-manager';
import { themeManager } from './utils/theme-manager';
import { useModal } from '@/composables';
import { useTabManagement } from '@/composables';
import { TAB_MANAGEMENT_KEY, OPEN_SSH_FORM_KEY, CLOSE_SSH_FORM_KEY, SHOW_SSH_FORM_KEY } from '@/types';
import { APP_EVENTS } from '@/constants';
import { eventBus } from '@/utils/event-bus';

const showSettings = ref(false);
provide('showSettings', showSettings);

// SSH连接表单管理
const { isOpen: showSSHForm, openModal: openSSHForm, closeModal: closeSSHForm } = useModal();
provide(SHOW_SSH_FORM_KEY, showSSHForm);
provide(OPEN_SSH_FORM_KEY, openSSHForm);
provide(CLOSE_SSH_FORM_KEY, closeSSHForm);

// 标签管理
const tabManagement = useTabManagement();
provide(TAB_MANAGEMENT_KEY, tabManagement);

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

  eventBus.on(APP_EVENTS.OPEN_SETTINGS, () => {
    showSettings.value = true;
  });

  eventBus.on(APP_EVENTS.CLOSE_DIALOG, () => {
    showSettings.value = false;
    closeSSHForm();
  });

  eventBus.on(APP_EVENTS.OPEN_SSH_FORM, () => {
    openSSHForm();
  });
});

onBeforeUnmount(() => {
  shortcutManager.unregisterAll();
});

// 处理SSH连接
const handleSSHConnect = (data: any) => {
  console.log('SSH连接数据:', data);
  // TODO: 实现实际的SSH连接逻辑
  closeSSHForm();
};

// 处理取消SSH连接
const handleSSHCancel = () => {
  closeSSHForm();
};
</script>

<template>
  <div
    id="app"
    class="app-wrapper"
  >
    <div class="app-root">
      <WindowTitleBar />
      <AppTabs />
      <AppContent />
    </div>
    
    <!-- SSH连接表单弹窗 -->
    <div
      v-if="showSSHForm"
      class="modal-overlay"
      @click.self="closeSSHForm"
    >
      <SSHConnectionForm 
        @connect="handleSSHConnect" 
        @cancel="handleSSHCancel" 
      />
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

/* Modal overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .modal-overlay {
    background: rgba(0, 0, 0, 0.7);
  }
}

:root.theme-dark .modal-overlay {
  background: rgba(0, 0, 0, 0.7);
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
    /* Chinese fonts for macOS */ 'PingFang SC',
    'Hiragino Sans GB',
    /* Chinese fonts for Windows */ 'Microsoft YaHei UI',
    'Microsoft YaHei',
    /* Chinese fonts for Linux */ 'WenQuanYi Micro Hei',
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