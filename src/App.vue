<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, provide } from 'vue';
import { useI18n } from 'vue-i18n';
import { v4 as uuidv4 } from 'uuid';
import WindowTitleBar from '@/components/layout/WindowTitleBar.vue';
import AppTabs from '@/components/layout/AppTabs.vue';
import AppContent from '@/components/layout/AppContent.vue';
import SSHConnectionForm from '@/components/ssh/SSHConnectionForm.vue';
import SettingsPanel from '@/components/settings/SettingsPanel.vue';
import WelcomeScreen from '@/components/common/WelcomeScreen.vue';
import {
  shortcutManager,
  PredefinedShortcuts,
} from '@/core/utils/shortcut-manager';
import { themeManager } from '@/core/utils/theme-manager';
import { useModal } from '@/composables';
import { useTabManagement } from '@/composables';
import { useSessionStore } from '@/features/session';
import {
  TAB_MANAGEMENT_KEY,
  OPEN_SSH_FORM_KEY,
  CLOSE_SSH_FORM_KEY,
  SHOW_SSH_FORM_KEY,
  SHOW_SETTINGS_KEY,
} from '@/core/types';
interface SSHConnectionFormData {
  name: string;
  host: string;
  port: number | null;
  username: string;
  password?: string;
  privateKey?: string;
  keyPassphrase?: string;
}
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';
import { createLogger } from '@/core/utils/logger';
import { TAB_TYPE } from '@/features/tabs';

const logger = createLogger('APP');

// Welcome screen state
const showWelcome = ref(localStorage.getItem('hasLaunched') !== 'true');

// Session management with Pinia
const sessionStore = useSessionStore();

// SSH connection form management
const {
  isOpen: showSSHForm,
  openModal: openSSHForm,
  closeModal: closeSSHForm,
} = useModal();
const isConnecting = ref(false);
const sshErrorMessage = ref<string | null>(null);

const { t } = useI18n({ useScope: 'global' });

provide(SHOW_SSH_FORM_KEY, showSSHForm);
provide(OPEN_SSH_FORM_KEY, () => {
  sshErrorMessage.value = null;
  isConnecting.value = false;
  openSSHForm();
});
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

  // Clean up all sessions using Pinia store
  sessionStore.cleanupAllSessions().catch(error => {
    logger.error('Error cleaning up sessions on app close', error);
  });
});

// Handle SSH connection with improved error handling
const handleSSHConnect = async (data: SSHConnectionFormData) => {
  logger.info('Initiating SSH connection', {
    name: data.name,
    host: data.host,
    port: data.port,
  });

  sshErrorMessage.value = null;
  isConnecting.value = true;

  // 1. Generate a unique session ID
  const sessionId = uuidv4();

  // 2. Initiate backend connection via Pinia store
  try {
    await sessionStore.createSSHSession(
      sessionId,
      sessionId, // Use sessionId as tabId for now
      data.name,
      data.host,
      data.port || 22,
      data.username,
      data.password || '',
      80, // Default columns
      24 // Default rows
    );
    logger.info('SSH session created successfully', { sessionId });

    // 3. Create and add a new tab ONLY after successful connection
    tabManagement.addTab({
      id: sessionId,
      label: data.name || data.host,
      type: TAB_TYPE.SSH,
      closable: true,
    });

    closeSSHForm();
  } catch (error) {
    logger.error('Failed to create SSH session', error);

    // Enhanced error handling with structured error information
    if (error instanceof Error) {
      const errorMessage = error.message;

      // Check for specific Tauri error patterns (structured SshError)
      if (typeof error === 'object' && error !== null) {
        const err = error as Record<string, any>;

        if (err.connectionFailed) {
          sshErrorMessage.value = `${t('ssh.errorConnectionFailed')}: ${err.connectionFailed.host}:${err.connectionFailed.port} - ${err.connectionFailed.reason}`;
        } else if (err.authenticationFailed) {
          sshErrorMessage.value = t('ssh.errorAuthenticationFailed');
        } else if (err.channelError) {
          sshErrorMessage.value = `${t('ssh.errorChannel')}: ${err.channelError}`;
        } else {
          sshErrorMessage.value = errorMessage;
        }
      } else {
        sshErrorMessage.value = errorMessage;
      }
    } else {
      sshErrorMessage.value = String(error);
    }
  } finally {
    isConnecting.value = false;
  }
};

// Handle SSH connection cancellation
const handleSSHCancel = () => {
  closeSSHForm();
};

// Handle settings panel events
const handleSettingsUpdate = (value: boolean) => {
  showSettings.value = value;
};

// Handle creating a new tab
const handleCreateTab = (tab: any) => {
  tabManagement.addTab(tab);
};
</script>

<template>
  <div id="app" class="app-wrapper">
    <div class="app-root">
      <WindowTitleBar />
      <AppTabs />
      <AppContent @create-tab="handleCreateTab" @connect="handleSSHConnect" />

      <!-- SSH connection form modal -->
      <div v-if="showSSHForm" class="modal-system-overlay">
        <div class="modal-system-panel">
          <SSHConnectionForm
            :is-loading="isConnecting"
            :error-message="sshErrorMessage"
            @connect="handleSSHConnect"
            @cancel="handleSSHCancel"
          />
        </div>
      </div>

      <!-- Settings panel modal -->
      <SettingsPanel
        :visible="showSettings"
        :use-teleport="false"
        @update:visible="handleSettingsUpdate"
      />

      <!-- Welcome screen for first launch -->
      <WelcomeScreen v-if="showWelcome" @complete="showWelcome = false" />
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
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  border-radius: var(--radius-2xl);
  overflow: hidden;
  background-color: var(--color-bg-primary);
  box-shadow:
    0 0 0 0.5px rgba(0, 0, 0, 0.1),
    var(--shadow-2xl);
  border: none;
  transition: all var(--transition-base);
  /* Use clip-path to force cropping and prevent black edges from rendering overflow */
  clip-path: inset(0 round var(--radius-2xl));
}

/* Fullscreen mode: remove rounded corners and borders */
@media (display-mode: fullscreen) {
  .app-root {
    border-radius: 0;
    border: none;
    clip-path: none;
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
  position: absolute;
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
  border-radius: inherit;
  overflow: hidden;
}

/* Modal content with enhanced styling for better visibility */
.modal-content {
  position: relative;
  /* Remove physical border and clip-path, switch to shadow simulation */
  border: none;
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.05),
    0 10px 40px rgba(0, 0, 0, 0.15);
  border-radius: var(--radius-lg);
  overflow: hidden;
  clip-path: none;
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
  /* 取消全局拖拽功能 */
  -webkit-app-region: no-drag;
}

div[role='region'] {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
  /* 确保这些区域也不可拖拽 */
  -webkit-app-region: no-drag;
}
</style>
