<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, provide } from 'vue';
// i18n not used in this file for progress messages (messages are plain English)
import { v4 as uuidv4 } from 'uuid';
import { invoke } from '@tauri-apps/api/core';
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
  server_name: string;
  addr: string;
  port: number | null;
  username: string;
  password: string;
  private_key_path: string;
  key_passphrase: string;
  save_session: boolean;
  groups?: string[];
  tags?: string[];
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
const sshFormMode = ref<'create' | 'edit'>('create');
const editingSessionId = ref<string | null>(null);

// Save form data for restoration on cancel
const savedSSHFormData = ref<SSHConnectionFormData | null>(null);

// Connection progress state
const showConnectionProgress = ref(false);
const connectionTime = ref(0);
let connectionTimerInterval: any = null;
const connectionProgress = ref(0);
const connectionCurrentStep = ref(0);
const connectionMessage = ref('');
const connectionStatus = ref<'connecting' | 'success' | 'error'>('connecting');
const connectionErrorMessage = ref('');
const connectionErrorTitle = ref('');

// no `t` used here

provide(SHOW_SSH_FORM_KEY, showSSHForm);
provide(OPEN_SSH_FORM_KEY, () => {
  sshErrorMessage.value = null;
  isConnecting.value = false;
  sshFormMode.value = 'create';
  editingSessionId.value = null;
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

  eventBus.on(APP_EVENTS.EDIT_SESSION, async (session: any) => {
    // Handle edit session event
    const credentials = await invoke<[string, string | null, string | null]>(
      'get_session_credentials',
      { sessionId: session.id }
    ).catch(() => [session.id, null, null]);

    sshFormMode.value = 'edit';
    editingSessionId.value = session.id;

    savedSSHFormData.value = {
      server_name: session.server_name,
      addr: session.addr,
      port: session.port,
      username: session.username,
      password: credentials[1] || '',
      private_key_path: session.private_key_path || '',
      key_passphrase: credentials[2] || '',
      save_session: true,
      groups: session.groups || [],
      tags: session.tags || [],
    };

    sshErrorMessage.value = null;
    isConnecting.value = false;
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
    name: data.server_name,
    host: data.addr,
    port: data.port,
  });

  // Save form data for later restoration
  savedSSHFormData.value = { ...data };

  sshErrorMessage.value = null;
  isConnecting.value = true;

  // Show progress bar inside the SSH form - do not close the form immediately
  showConnectionProgress.value = true;
  connectionTime.value = 0;

  if (connectionTimerInterval) clearInterval(connectionTimerInterval);
  connectionTimerInterval = setInterval(() => {
    connectionTime.value++;
    // If it takes more than 30 seconds, it's probably timed out (handled by backend too)
    if (connectionTime.value >= 30 && connectionStatus.value === 'connecting') {
      // The backend has its own 30s timeout, but we can reflect it here if needed
    }
  }, 1000);

  connectionProgress.value = 0;
  connectionCurrentStep.value = 0;
  connectionMessage.value = 'Establishing SSH connection';
  connectionStatus.value = 'connecting';
  connectionErrorMessage.value = '';
  connectionErrorTitle.value = '';

  // Process any new groups or tags first
  const processMetadata = async () => {
    const finalGroupIds = [...(data.groups || [])];
    const finalTagIds = [...(data.tags || [])];

    // Handle new groups
    for (let i = 0; i < finalGroupIds.length; i++) {
      if (finalGroupIds[i].startsWith('new:')) {
        const name = finalGroupIds[i].substring(4);
        try {
          const id = await invoke<string>('add_group', { name });
          finalGroupIds[i] = id;
        } catch (error) {
          logger.error(`Failed to create group: ${name}`, error);
        }
      }
    }

    // Handle new tags
    for (let i = 0; i < finalTagIds.length; i++) {
      if (finalTagIds[i].startsWith('new:')) {
        const name = finalTagIds[i].substring(4);
        try {
          const id = await invoke<string>('add_tag', { name });
          finalTagIds[i] = id;
        } catch (error) {
          logger.error(`Failed to create tag: ${name}`, error);
        }
      }
    }

    return { groupIds: finalGroupIds, tagIds: finalTagIds };
  };

  // Check if we're in edit mode
  if (sshFormMode.value === 'edit' && editingSessionId.value) {
    try {
      const authType = data.password ? 'password' : 'key';

      const { groupIds, tagIds } = await processMetadata();

      // Update the session metadata and credentials in one go
      // This avoids creating duplicate entries in the sessions table
      await invoke('save_session_with_credentials', {
        id: editingSessionId.value,
        addr: data.addr,
        port: data.port || 22,
        serverName: data.server_name,
        username: data.username,
        authType: authType,
        privateKeyPath: data.private_key_path || null,
        password: data.password || null,
        keyPassphrase: data.key_passphrase || null,
        groupIds: groupIds.length > 0 ? groupIds : null,
        tagIds: tagIds.length > 0 ? tagIds : null,
      });

      // Update timestamp to mark as recent
      await invoke('update_session_timestamp', { id: editingSessionId.value });

      connectionProgress.value = 100;
      connectionCurrentStep.value = 1;
      connectionStatus.value = 'success';
      connectionMessage.value = 'Session updated successfully';

      logger.info('SSH session updated successfully', {
        sessionId: editingSessionId.value,
        name: data.server_name,
        host: data.addr,
      });

      // Auto-close after 1 second
      await new Promise(resolve => setTimeout(resolve, 1000));
      closeSSHForm();
      showConnectionProgress.value = false;

      // Emit event to refresh the session list
      eventBus.emit(APP_EVENTS.SESSION_SAVED, {
        name: data.server_name,
        host: data.addr,
        port: data.port || 22,
      });

      // Reset form mode
      sshFormMode.value = 'create';
      editingSessionId.value = null;
    } catch (error) {
      logger.error('Failed to update session', error);
      connectionStatus.value = 'error';
      connectionProgress.value = 100;
      connectionErrorTitle.value = 'Failed to update session';
      connectionErrorMessage.value = String(error);
    }
    return;
  }

  // 1. Generate a unique session ID
  const sessionId = uuidv4();

  // 2. Initiate backend connection via Pinia store
  try {
    // Simulate step transitions for better UX
    setTimeout(() => {
      connectionCurrentStep.value = 1;
      connectionProgress.value = 30;
      connectionMessage.value = 'Authenticating user';
    }, 800);

    await sessionStore.createSSHSession(
      sessionId,
      sessionId, // Use sessionId as tabId for now
      data.server_name,
      data.addr,
      data.port || 22,
      data.username,
      data.password || '',
      80, // Default columns
      24 // Default rows
    );
    logger.info('SSH session created successfully', { sessionId });

    // 1.5. Save session to database AFTER successful connection if user requested it
    if (data.save_session) {
      try {
        const authType = data.password ? 'password' : 'key';

        const { groupIds, tagIds } = await processMetadata();

        logger.info(
          'Attempting to save session after successful connection...',
          {
            name: data.server_name,
            host: data.addr,
            authType,
            hasGroups: groupIds.length > 0,
            hasTags: tagIds.length > 0,
            groups: groupIds,
            tags: tagIds,
          }
        );

        const savePayload = {
          id: null, // New connection, id is null
          addr: data.addr,
          port: data.port || 22,
          serverName: data.server_name,
          username: data.username,
          authType: authType,
          privateKeyPath: data.private_key_path || null,
          password: data.password || null,
          keyPassphrase: data.key_passphrase || null,
          groupIds: groupIds.length > 0 ? groupIds : null,
          tagIds: tagIds.length > 0 ? tagIds : null,
        };

        const savedSessionId = await invoke(
          'save_session_with_credentials',
          savePayload
        );

        // Update timestamp for new session too
        if (savedSessionId) {
          await invoke('update_session_timestamp', { id: savedSessionId });
        }

        logger.info('SSH session saved to database', {
          sessionId: savedSessionId,
          name: data.server_name,
          host: data.addr,
        });

        // Emit event to notify other components that a new session has been saved
        eventBus.emit(APP_EVENTS.SESSION_SAVED, {
          name: data.server_name,
          host: data.addr,
          port: data.port || 22,
        });
      } catch (saveError) {
        logger.error('Failed to save session to database', saveError);
        // We don't throw error here to not fail the already established connection
      }
    }

    connectionCurrentStep.value = 2;
    connectionProgress.value = 70;
    connectionMessage.value = 'Initializing terminal';

    // Final step completion
    connectionProgress.value = 100;
    connectionCurrentStep.value = 3;
    connectionStatus.value = 'success';
    connectionMessage.value = 'Connection established successfully';

    // Keep progress bar visible for a brief moment before closing and opening the tab
    setTimeout(() => {
      // Close the SSH form entirely (including progress bar)
      closeSSHForm();
      showConnectionProgress.value = false;

      // 3. Create and add a new tab AFTER the form is closed
      // Use a small delay to allow the modal to disappear visually
      setTimeout(() => {
        tabManagement.addTab({
          id: sessionId,
          label: data.server_name || data.addr,
          type: TAB_TYPE.SSH,
          closable: true,
        });
      }, 100);
    }, 500);
  } catch (error) {
    logger.error('Failed to create SSH session', error);

    // Set error state in progress bar
    connectionStatus.value = 'error';
    connectionProgress.value = 0;
    connectionErrorTitle.value = 'Connection Failed';
    connectionMessage.value = 'Failed to establish connection';

    // Parse error message
    let errorDetails = '';

    // Handle both JS Errors and structured objects from Tauri
    if (typeof error === 'object' && error !== null) {
      const err = error as Record<string, any>;

      // Check for specific Tauri error patterns (structured SshError)
      if (err.connectionFailed) {
        errorDetails = `Connection failed: ${err.connectionFailed.host}:${err.connectionFailed.port} - ${err.connectionFailed.reason}`;
      } else if (err.authenticationFailed) {
        errorDetails =
          'Authentication failed. Please check your username and password.';
      } else if (err.channelError) {
        errorDetails = `Channel error: ${err.channelError}`;
      } else if (err.message) {
        // Standard JS Error or object with message property
        errorDetails = String(err.message);
      } else {
        // Fallback for other objects: try JSON stringify to see content
        try {
          // Avoid [object Object]
          const json = JSON.stringify(error);
          errorDetails = json === '{}' ? 'Unknown error' : json;
        } catch (e) {
          // If stringify failed, fall back to string conversion
          errorDetails = String(error);
          logger.debug('JSON stringify failed while formatting error', e);
        }
      }
    } else {
      errorDetails = String(error);
    }

    // Clean up error message if it is literally wrapping a string like "Error: ..."
    if (errorDetails.startsWith('"') && errorDetails.endsWith('"')) {
      try {
        errorDetails = JSON.parse(errorDetails);
      } catch (e) {
        // ignore parse errors and keep the raw string
        logger.debug('Failed to JSON.parse errorDetails', e);
      }
    }

    connectionErrorMessage.value = errorDetails;
    sshErrorMessage.value = errorDetails;
  } finally {
    isConnecting.value = false;
    if (connectionTimerInterval) {
      clearInterval(connectionTimerInterval);
      connectionTimerInterval = null;
    }
  }
};

// Handle connection progress bar close
const handleConnectionProgressClose = () => {
  showConnectionProgress.value = false;

  // Show SSH form again with saved data
  openSSHForm();
};

// Handle connection progress bar retry
const handleConnectionProgressRetry = () => {
  // Close progress bar and reopen SSH form
  showConnectionProgress.value = false;
  openSSHForm();
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
      <template v-if="!showWelcome">
        <WindowTitleBar />
        <AppTabs />
        <AppContent @create-tab="handleCreateTab" @connect="handleSSHConnect" />
      </template>

      <!-- SSH connection form modal -->
      <div v-if="showSSHForm" class="modal-system-overlay">
        <div class="modal-system-panel">
          <SSHConnectionForm
            :is-loading="isConnecting"
            :error-message="sshErrorMessage"
            :initial-data="savedSSHFormData || undefined"
            :show-progress="showConnectionProgress"
            :connection-status="connectionStatus"
            :connection-progress="connectionProgress"
            :connection-current-step="connectionCurrentStep"
            :connection-message="connectionMessage"
            :connection-time="connectionTime"
            :connection-error-title="connectionErrorTitle"
            :connection-error-message="connectionErrorMessage"
            @connect="handleSSHConnect"
            @cancel="handleSSHCancel"
            @retry="handleConnectionProgressRetry"
            @close-progress="handleConnectionProgressClose"
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
