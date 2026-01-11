<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, provide } from 'vue';
import WindowTitleBar from '@/components/layout/WindowTitleBar.vue';
import AppTabs from '@/components/layout/AppTabs.vue';
import AppContent from '@/components/layout/AppContent.vue';
import SSHConnectionForm from '@/components/ssh/SSHConnectionForm.vue';
import SettingsPanel from '@/components/settings/SettingsPanel.vue';
import { shortcutManager, PredefinedShortcuts } from './utils/shortcut-manager';
import { themeManager } from './utils/theme-manager';
import { useModal } from '@/composables';
import { useTabManagement } from '@/composables';
import {
  TAB_MANAGEMENT_KEY,
  OPEN_SSH_FORM_KEY,
  CLOSE_SSH_FORM_KEY,
  SHOW_SSH_FORM_KEY,
  SHOW_SETTINGS_KEY,
} from '@/types';
interface SSHConnectionFormData {
  name: string;
  host: string;
  port: number | null;
  username: string;
  password: string;
  privateKey: string;
  keyPassphrase: string;
}
import { APP_EVENTS } from '@/constants';
import { eventBus } from '@/utils/event-bus';

// SSH connection form management
const {
  isOpen: showSSHForm,
  openModal: openSSHForm,
  closeModal: closeSSHForm,
} = useModal();
provide(SHOW_SSH_FORM_KEY, showSSHForm);
provide(OPEN_SSH_FORM_KEY, openSSHForm);
provide(CLOSE_SSH_FORM_KEY, closeSSHForm);

// Settings panel management
const showSettings = ref(false);
const openSettings = () => {
  showSettings.value = true;
};
const closeSettings = () => {
  showSettings.value = false;
};
provide(SHOW_SETTINGS_KEY, showSettings);

// Tab management
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
    openSettings();
  });

  eventBus.on(APP_EVENTS.CLOSE_DIALOG, () => {
    closeSettings();
    closeSSHForm();
  });

  eventBus.on(APP_EVENTS.OPEN_SSH_FORM, () => {
    openSSHForm();
  });
});

onBeforeUnmount(() => {
  shortcutManager.unregisterAll();
});

// Handle SSH connection
const handleSSHConnect = (data: SSHConnectionFormData) => {
  console.log('SSH connection data:', data);
  // TODO: Implement actual SSH connection logic
  closeSSHForm();
};

// Handle SSH connection cancellation
const handleSSHCancel = () => {
  closeSSHForm();
};

// Handle settings panel events
const handleSettingsUpdate = (value: boolean) => {
  showSettings.value = value;
};
</script>

<template>
  <div id="app" class="app-wrapper">
    <div class="app-root">
      <WindowTitleBar />
      <AppTabs />
      <AppContent />
    </div>

    <!-- SSH connection form modal -->
    <div v-if="showSSHForm" class="modal-overlay" @click.self="closeSSHForm">
      <div class="modal-content">
        <SSHConnectionForm
          @connect="handleSSHConnect"
          @cancel="handleSSHCancel"
        />
      </div>
    </div>

    <!-- Settings panel modal -->
    <div v-if="showSettings" class="modal-overlay" @click.self="closeSettings">
      <div class="modal-content">
        <SettingsPanel
          :visible="showSettings"
          :use-teleport="false"
          @update:visible="handleSettingsUpdate"
        />
      </div>
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

/* Modal overlay - removing black overlay for desktop app but keeping focus */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(
    2px
  ); /* Subtle blur to distinguish modal from background */
}

/* Modal content with enhanced styling for better visibility */
.modal-content {
  position: relative;
  box-shadow:
    0 10px 40px rgba(0, 0, 0, 0.15),
    0 0 0 1px rgba(0, 0, 0, 0.05);
  border-radius: var(--radius-lg);
  animation: modal-appear 0.2s ease-out forwards;
}

@keyframes modal-appear {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
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
