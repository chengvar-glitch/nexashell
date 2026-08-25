<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, provide } from 'vue';
import { v4 as uuidv4 } from 'uuid';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import WindowTitleBar from '@/components/layout/WindowTitleBar.vue';
import AppTabs from '@/components/layout/AppTabs.vue';
import AppContent from '@/components/layout/AppContent.vue';
import SSHConnectionForm from '@/components/connections/SSHConnectionForm.vue';
import SettingsPanel from '@/components/settings/SettingsPanel.vue';
import WelcomeScreen from '@/components/common/WelcomeScreen.vue';
import CommandPalette from '@/components/palette/CommandPalette.vue';
import {
  shortcutManager,
  PredefinedShortcuts,
} from '@/core/utils/shortcut-manager';
import { themeManager } from '@/core/utils/theme-manager';
import { useModal, useTabManagement } from '@/composables';
import { useSessionStore } from '@/features/session';
import { tunnelApi } from '@/features/tunnel';
import type { SavedSession, SavedSessionDisplay } from '@/features/session/types';
import {
  TAB_MANAGEMENT_KEY,
  OPEN_SSH_FORM_KEY,
  CLOSE_SSH_FORM_KEY,
  SHOW_SSH_FORM_KEY,
  SHOW_SETTINGS_KEY,
} from '@/core/types';
interface SSHConnectionFormData {
  id?: string; // session ID, set when editing existing session
  server_name: string;
  addr: string;
  port: number | null;
  username: string;
  // Optional: omitted (undefined) in edit mode when the user left it blank so
  // the backend keeps the stored ciphertext ("unchanged"); `null` means the
  // user explicitly asked to clear the stored value.
  password?: string | null;
  private_key_path: string;
  key_passphrase?: string | null;
  save_session: boolean;
  groups?: string[];
  tags?: string[];
  /** True when the user explicitly asked to clear any stored credentials. */
  clearCredentials?: boolean;
}
import { APP_EVENTS } from '@/core/constants';
import { eventBus } from '@/core/utils/event-bus';
import { createLogger } from '@/core/utils/logger';
import { TAB_TYPE } from '@/features/tabs';

import { isWindows } from '@/core/utils/platform/platform-detection';

const logger = createLogger('APP');
const { t } = useI18n();

// Platform state
const isWindowsState = ref(false);

// Global contextmenu handler reference so we can remove it on unmount
let __globalContextMenuHandler: ((e: MouseEvent) => void) | null = null;

// EventBus handlers registered in onMounted, removed in onBeforeUnmount
const __eventOffFns: Array<() => void> = [];

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

// In-flight connection tracking so Cancel can abort a connecting session
let activeConnectionId: string | null = null;
let connectionCancelled = false;

// Connection progress state
const showConnectionProgress = ref(false);
const connectionTime = ref(0);
let connectionTimerInterval: ReturnType<typeof setInterval> | null = null;

// Two timer pools so the two purposes never interfere:
//  - pendingTimeouts: simulated step timers during connection — cleared on
//    failure and on unmount.
//  - successTimeouts: post-success actions (close form, open tab) — must
//    survive a failure of a LATER connection attempt, and are cleared when
//    they fire or the component unmounts.
const pendingTimeouts: ReturnType<typeof setTimeout>[] = [];
const successTimeouts: ReturnType<typeof setTimeout>[] = [];

const clearPendingTimeouts = () => {
  pendingTimeouts.forEach(t => clearTimeout(t));
  pendingTimeouts.length = 0;
};

const clearSuccessTimeouts = () => {
  successTimeouts.forEach(t => clearTimeout(t));
  successTimeouts.length = 0;
};
const connectionProgress = ref(0);
const connectionCurrentStep = ref(0);
const connectionMessage = ref('');
const connectionStatus = ref<'connecting' | 'success' | 'error'>('connecting');
const connectionErrorMessage = ref('');
const connectionErrorTitle = ref('');

// no `t` used here

// Reset all SSH-form transient state so a fresh open starts in "create" mode.
// Shared by the provided OPEN_SSH_FORM handler and the APP_EVENTS.OPEN_SSH_FORM
// event handler so Ctrl+T after cancel/edit never reopens a stale "edit" form
// that would overwrite the original session on save.
const resetSSHFormState = () => {
  sshErrorMessage.value = null;
  isConnecting.value = false;
  sshFormMode.value = 'create';
  editingSessionId.value = null;
  savedSSHFormData.value = null;
  showConnectionProgress.value = false;
};

const openSSHFormReset = () => {
  resetSSHFormState();
  openSSHForm();
};

provide(SHOW_SSH_FORM_KEY, showSSHForm);
provide(OPEN_SSH_FORM_KEY, openSSHFormReset);
provide(CLOSE_SSH_FORM_KEY, closeSSHForm);

// Settings panel management
const showSettings = ref(false);
const settingsInitialSection = ref('appearance');
const openSettings = (section?: string) => {
  settingsInitialSection.value = section || 'appearance';
  showSettings.value = true;
};
const closeSettings = () => {
  showSettings.value = false;
};

// Command palette (snippet library) management
const showCommandPalette = ref(false);
const openCommandPalette = () => {
  showCommandPalette.value = true;
};
const closeCommandPalette = (value: boolean) => {
  showCommandPalette.value = value;
};
provide(SHOW_SETTINGS_KEY, showSettings);

// Transient toast for blocked actions (e.g. the per-tab split limit).
const toastMessage = ref('');
let toastTimer: ReturnType<typeof setTimeout> | null = null;
const showToast = (message: string) => {
  toastMessage.value = message;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toastMessage.value = '';
    toastTimer = null;
  }, 2200);
};

// Tab management
const tabManagement = useTabManagement();
provide(TAB_MANAGEMENT_KEY, tabManagement);

onMounted(async () => {
  isWindowsState.value = await isWindows();

  // Initialize theme system
  themeManager.initialize();

  shortcutManager.register(PredefinedShortcuts.QUIT_APP);
  shortcutManager.register(PredefinedShortcuts.OPEN_SETTINGS);
  shortcutManager.register(PredefinedShortcuts.NEW_LOCAL_TAB);
  shortcutManager.register(PredefinedShortcuts.NEW_SSH_TAB);
  shortcutManager.register(PredefinedShortcuts.CLOSE_CURRENT_TAB);
  shortcutManager.register(PredefinedShortcuts.FOCUS_SEARCH);
  shortcutManager.register(PredefinedShortcuts.CLOSE_DIALOG);
  shortcutManager.register(PredefinedShortcuts.SPLIT_VERTICAL);
  shortcutManager.register(PredefinedShortcuts.SPLIT_HORIZONTAL);
  shortcutManager.register(PredefinedShortcuts.COMMAND_PALETTE);

  // All eventBus handlers are collected so they can be removed on unmount,
  // preventing leaks and duplicate handlers during HMR.
  const offFns = __eventOffFns;

  offFns.push(
    eventBus.on(APP_EVENTS.OPEN_SETTINGS, (args: unknown) => {
      const payload = args as { section?: string } | undefined;
      openSettings(payload?.section);
    })
  );

  offFns.push(
    eventBus.on(APP_EVENTS.CLOSE_DIALOG, () => {
      closeSettings();
      closeSSHForm();
    })
  );

  offFns.push(
    eventBus.on(APP_EVENTS.SPLIT_VERTICAL, () => {
      if (tabManagement.splitActivePane('vertical') === 'limit') {
        showToast(t('pane.splitLimit'));
      }
    })
  );

  offFns.push(
    eventBus.on(APP_EVENTS.SPLIT_HORIZONTAL, () => {
      if (tabManagement.splitActivePane('horizontal') === 'limit') {
        showToast(t('pane.splitLimit'));
      }
    })
  );

  offFns.push(
    eventBus.on(APP_EVENTS.OPEN_SSH_FORM, () => {
      openSSHFormReset();
    })
  );

  offFns.push(
    eventBus.on(APP_EVENTS.COMMAND_PALETTE, () => {
      openCommandPalette();
    })
  );

  offFns.push(
    eventBus.on(APP_EVENTS.EDIT_SESSION, (async (session: unknown) => {
      const payload = session as SavedSessionDisplay | null;
      logger.debug('Handling EDIT_SESSION event', payload?.id);
      if (!payload) {
        logger.error('EDIT_SESSION session is null');
        return;
      }

      // 1. Initial form state with known info (no password/passphrase yet)
      sshFormMode.value = 'edit';
      editingSessionId.value = payload.id;

      const initialFormData: SSHConnectionFormData = {
        id: payload.id,
        server_name: payload.server_name || '',
        addr: payload.addr || '',
        port: payload.port || 22,
        username: payload.username || '',
        password: '',
        private_key_path: payload.private_key_path || '',
        key_passphrase: '',
        save_session: true,
        groups: payload.group_ids ? [...payload.group_ids] : [],
        tags: payload.tag_ids ? [...payload.tag_ids] : [],
      };

      savedSSHFormData.value = initialFormData;
      sshErrorMessage.value = null;
      isConnecting.value = false;

      // 2. Open form immediately to respond to user click
      logger.debug('Calling openSSHForm()');
      openSSHForm();

      // 3. Fetch credentials in background
      try {
        logger.debug('Fetching credentials in background for', payload.id);
        const credentials = await invoke<[string, string | null, string | null]>(
          'get_session_credentials',
          { sessionId: payload.id }
        );

        // 4. If form is still open and we're editing the same session, update sensitive fields
        if (showSSHForm.value && editingSessionId.value === payload.id) {
          logger.debug('Updating sensitive fields in background');
          savedSSHFormData.value = {
            ...initialFormData,
            password: credentials[1] || '',
            key_passphrase: credentials[2] || '',
          };
        }
      } catch (error) {
        logger.error('Failed to fetch credentials in background', error);
      }
    }) as (...args: unknown[]) => void)
  );

  offFns.push(
    eventBus.on(APP_EVENTS.CONNECT_SESSION, (async (args: unknown) => {
      const session = args as SavedSession;
      if (!session) return;

      try {
        sshFormMode.value = 'create';
        editingSessionId.value = session.id;

        const credentials = await invoke<[string, string | null, string | null]>(
          'get_session_credentials',
          { sessionId: session.id }
        ).catch(() => [session.id, null, null]);

        const connectData: SSHConnectionFormData = {
          id: session.id,
          server_name: session.server_name,
          addr: session.addr,
          port: session.port,
          username: session.username,
          password: credentials[1] || '',
          private_key_path: session.private_key_path || '',
          key_passphrase: credentials[2] || '',
          save_session: false,
          groups: [],
          tags: [],
        };

        await invoke('update_session_timestamp', { id: session.id }).catch(err =>
          logger.error('Failed to update timestamp', err)
        );
        eventBus.emit(APP_EVENTS.SESSION_SAVED);

        savedSSHFormData.value = connectData;
        openSSHForm();
        handleSSHConnect(connectData);
      } catch (error) {
        logger.error('Failed to connect to saved session', error);
      }
    }) as (...args: unknown[]) => void)
  );

  // Global right-click handling: suppress the browser's default context menu
  // (Inspect Element / Reload etc.) across the whole window — it reads as
  // "web" on a desktop app. Only text-editing elements keep the native menu
  // (copy/paste, incl. password fields) and the terminal keeps its own custom
  // menu (its handler runs closer to the target and preventDefaults first).
  // Unlike before, this applies in dev too — the menu is noise everywhere.
  __globalContextMenuHandler = (e: MouseEvent) => {
    const target = e.target as HTMLElement | null;
    if (!target) return;

    // Elements that should keep a native/context menu
    const interactiveSelector = [
      'a',
      'input',
      'textarea',
      'select',
      '[contenteditable]',
      '.terminal-container', // owns its own right-click menu
    ].join(',');

    if (target.closest(interactiveSelector)) {
      // clicked on an interactive element — allow default
      return;
    }

    // otherwise treat as blank area and prevent browser menu
    e.preventDefault();
    e.stopPropagation();
  };

  window.addEventListener('contextmenu', __globalContextMenuHandler);
});

onBeforeUnmount(() => {
  shortcutManager.unregisterAll();

  // Remove all eventBus handlers registered in onMounted
  __eventOffFns.forEach(fn => fn());
  __eventOffFns.length = 0;

  // Cancel any pending connection timers before the component is destroyed
  clearPendingTimeouts();
  clearSuccessTimeouts();
  if (connectionTimerInterval) {
    clearInterval(connectionTimerInterval);
    connectionTimerInterval = null;
  }
  if (toastTimer) {
    clearTimeout(toastTimer);
    toastTimer = null;
  }

  // Clean up all sessions using Pinia store
  sessionStore.cleanupAllSessions().catch(error => {
    logger.error('Error cleaning up sessions on app close', error);
  });
  if (__globalContextMenuHandler) {
    window.removeEventListener('contextmenu', __globalContextMenuHandler);
    __globalContextMenuHandler = null;
  }
});

// Process any new groups or tags first
const processMetadata = async (data: SSHConnectionFormData) => {
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
  }, 1000);

  connectionProgress.value = 0;
  connectionCurrentStep.value = 0;
  connectionMessage.value = t('connection.establishingSSH');
  connectionStatus.value = 'connecting';
  connectionErrorMessage.value = '';
  connectionErrorTitle.value = '';

  // 1. Generate a unique session ID for the RUNTIME terminal session
  const sessionId = uuidv4();
  activeConnectionId = sessionId;
  connectionCancelled = false;

  // 2. Initiate backend connection via Pinia store
  try {
    // Simulate step transitions for better UX
    pendingTimeouts.push(
      setTimeout(() => {
        connectionCurrentStep.value = 1;
        connectionProgress.value = 30;
        connectionMessage.value = t('connection.authenticating');
      }, 800)
    );

    await sessionStore.createSSHSession(
      sessionId,
      sessionId, // Use sessionId as tabId for now
      data.server_name,
      data.addr,
      data.port || 22,
      data.username,
      data.password || '',
      data.private_key_path || null,
      data.key_passphrase || null,
      80, // Default columns
      24 // Default rows
    );
    logger.info('SSH session created successfully', { sessionId });

    // Best-effort auto-start of persisted tunnels for this runtime session.
    // Failures are non-fatal; the user can still start tunnels from the panel.
    try {
      await tunnelApi.startSessionTunnels(sessionId);
    } catch (e) {
      logger.warn('Auto-start tunnels failed', e);
    }

    // User cancelled while the backend was connecting — tear down immediately
    if (connectionCancelled) {
      activeConnectionId = null;
      await sessionStore
        .disconnectSession(sessionId)
        .catch(e => logger.error('Failed to disconnect cancelled session', e));
      throw new Error('cancelled');
    }
    activeConnectionId = null;

    // 1.5. Save or update session in database AFTER successful connection
    if (data.id || (data.save_session && sshFormMode.value === 'create')) {
      try {
        const authType = data.password ? 'password' : 'key';

        const { groupIds, tagIds } = await processMetadata(data);

        logger.info(
          data.id ? 'Updating existing session...' : 'Saving new session...',
          {
            id: data.id || 'new',
            name: data.server_name,
            host: data.addr,
            authType,
          }
        );

        const savePayload = {
          id: data.id || null,
          addr: data.addr,
          port: data.port || 22,
          serverName: data.server_name,
          username: data.username,
          authType: authType,
          privateKeyPath: data.private_key_path || null,
          password: data.password || null,
          keyPassphrase: data.key_passphrase || null,
          clearCredentials: !!data.clearCredentials,
          groupIds: groupIds.length > 0 ? groupIds : null,
          tagIds: tagIds.length > 0 ? tagIds : null,
        };

        const resultId = await invoke<string>(
          'save_session_with_credentials',
          savePayload
        );

        // Update timestamp for recency tracking
        const timestampId = data.id || resultId;
        if (timestampId) {
          await invoke('update_session_timestamp', { id: timestampId }).catch(
            e => logger.error('Failed to update timestamp', e)
          );
        }

        logger.info('SSH session persistence completed', {
          sessionId: timestampId,
          hadId: !!data.id,
        });

        // Emit event to notify other components to refresh lists
        eventBus.emit(APP_EVENTS.SESSION_SAVED);
      } catch (saveError) {
        logger.error('Failed to persist session to database', saveError);
        // We don't throw error here to not fail the already established terminal session
      }
    }

    // Connection succeeded — cancel any remaining simulated step timers so a
    // fast connection can't be overwritten by the "authenticating" step.
    clearPendingTimeouts();

    connectionCurrentStep.value = 2;
    connectionProgress.value = 70;
    connectionMessage.value = t('connection.initializingTerminal');

    // Final step completion
    connectionProgress.value = 100;
    connectionCurrentStep.value = 3;
    connectionStatus.value = 'success';
    connectionMessage.value = t('connection.connectionEstablished');

    // Keep progress bar visible for a brief moment before closing and opening
    // the tab. These timers live in successTimeouts so they are NOT cleared by
    // a failure of a later connection attempt.
    successTimeouts.push(
      setTimeout(() => {
        // Close the SSH form entirely (including progress bar)
        closeSSHForm();
        showConnectionProgress.value = false;

        // Reset form mode and internal state
        sshFormMode.value = 'create';
        editingSessionId.value = null;

        // 3. Create and add a new tab AFTER the form is closed
        // Use a small delay to allow the modal to disappear visually
        successTimeouts.push(
          setTimeout(() => {
            const tabId = uuidv4();
            tabManagement.addTab({
              id: tabId,
              label: data.server_name || data.addr,
              type: TAB_TYPE.SSH,
              closable: true,
              panes: [{ id: sessionId, type: 'ssh' }],
            });
            tabManagement.setActivePane(sessionId);
          }, 100)
        );
      }, 500)
    );
  } catch (error) {
    // User-cancelled connections must not show an error UI
    if (connectionCancelled || String(error) === 'Error: cancelled') {
      connectionCancelled = false;
      return;
    }
    logger.error('Failed to create SSH session', error);

    // Set error state in progress bar
    connectionStatus.value = 'error';
    connectionProgress.value = 0;
    connectionErrorTitle.value = t('connection.connectionFailed');
    connectionMessage.value = t('connection.connectionError');

    // Parse error message
    let errorDetails = '';

    // Handle both JS Errors and structured objects from Tauri
    if (typeof error === 'object' && error !== null) {
      const err = error as Record<string, unknown>;

      // Check for specific Tauri error patterns (structured SshError)
      if (err.connectionFailed) {
        const cf = err.connectionFailed as {
          host: string;
          port: number;
          reason: string;
        };
        errorDetails = `${t('connection.connectionFailed')}: ${cf.host}:${cf.port} - ${cf.reason}`;
      } else if (err.authenticationFailed) {
        errorDetails = t('ssh.errorAuthenticationFailed');
      } else if (err.channelError) {
        errorDetails = `${t('ssh.errorChannel')}: ${String(err.channelError)}`;
      } else if (err.message) {
        // Standard JS Error or object with message property
        errorDetails = String(err.message);
      } else {
        // Fallback for other objects: try JSON stringify to see content
        try {
          // Avoid [object Object]
          const json = JSON.stringify(error);
          errorDetails = json === '{}' ? t('connection.connectionError') : json;
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

    // Connection failed — clear simulated step timers; the success-path
    // timers (e.g. closing the form) must be preserved so they can fire.
    clearPendingTimeouts();
  } finally {
    isConnecting.value = false;
    if (connectionTimerInterval) {
      clearInterval(connectionTimerInterval);
      connectionTimerInterval = null;
    }
  }
};

// Handle "Save Only" functionality
const handleSSHSave = async (data: SSHConnectionFormData) => {
  logger.info('Performing Save Only', {
    name: data.server_name,
    host: data.addr,
    hasId: !!data.id,
  });

  try {
    const authType = data.password ? 'password' : 'key';
    const { groupIds, tagIds } = await processMetadata(data);

    const savePayload = {
      id: data.id || null,
      addr: data.addr,
      port: data.port || 22,
      serverName: data.server_name,
      username: data.username,
      authType: authType,
      privateKeyPath: data.private_key_path || null,
      password: data.password || null,
      keyPassphrase: data.key_passphrase || null,
      clearCredentials: !!data.clearCredentials,
      groupIds: groupIds.length > 0 ? groupIds : null,
      tagIds: tagIds.length > 0 ? tagIds : null,
    };

    const resultId = await invoke<string>(
      'save_session_with_credentials',
      savePayload
    );

    const timestampId = data.id || resultId;
    if (timestampId) {
      await invoke('update_session_timestamp', { id: timestampId });
    }

    logger.info('SSH session saved via Save Only', {
      sessionId: timestampId,
    });

    // Emit event to notify other components to refresh lists
    eventBus.emit(APP_EVENTS.SESSION_SAVED, {
      name: data.server_name,
      host: data.addr,
      port: data.port || 22,
    });

    // Close form as we're done
    closeSSHForm();
    sshFormMode.value = 'create';
    editingSessionId.value = null;
  } catch (error) {
    logger.error('Failed to save session via Save Only', error);
    sshErrorMessage.value = String(error);
  }
};

// Handle connection progress bar close
const handleConnectionProgressClose = () => {
  cancelInFlightConnection();

  // Show SSH form again with saved data
  openSSHForm();
};

// Handle connection progress bar retry
const handleConnectionProgressRetry = () => {
  cancelInFlightConnection();

  // Close progress bar and reopen SSH form
  openSSHForm();
};

// Handle SSH connection cancellation
const handleSSHCancel = () => {
  if (isConnecting.value) {
    cancelInFlightConnection();
  }
  closeSSHForm();
};

/**
 * Aborts an in-flight SSH connection: flags the flow to stop, disconnects the
 * backend session (best effort — it may not exist yet), and resets progress UI.
 */
const cancelInFlightConnection = () => {
  connectionCancelled = true;
  clearPendingTimeouts();

  if (activeConnectionId) {
    const id = activeConnectionId;
    activeConnectionId = null;
    sessionStore
      .disconnectSession(id)
      .catch(e => logger.error('Failed to cancel in-flight connection', e));
  }

  isConnecting.value = false;
  showConnectionProgress.value = false;
  if (connectionTimerInterval) {
    clearInterval(connectionTimerInterval);
    connectionTimerInterval = null;
  }
};

// Handle settings panel events
const handleSettingsUpdate = (value: boolean) => {
  showSettings.value = value;
};

// Handle creating a new tab
const handleCreateTab = (tab: import('@/features/tabs/types').Tab) => {
  tabManagement.addTab(tab);
};
</script>

<template>
  <div id="app" class="app-wrapper">
    <div class="app-root" :class="{ 'is-windows': isWindowsState }">
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
            @save="handleSSHSave"
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
        :initial-section="settingsInitialSection"
        @update:visible="handleSettingsUpdate"
      />

      <!-- Welcome screen for first launch -->
      <WelcomeScreen v-if="showWelcome" @complete="showWelcome = false" />

      <!-- Command palette (snippet library) -->
      <CommandPalette
        :visible="showCommandPalette"
        @update:visible="closeCommandPalette"
      />

      <!-- Transient toast (blocked actions, e.g. split pane limit) -->
      <Transition name="toast">
        <div v-if="toastMessage" class="app-toast" role="status">
          {{ toastMessage }}
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.app-wrapper {
  width: 100vw;
  height: 100vh;
  padding: 0;
  background-color: var(--color-bg-primary);
  display: flex;
  flex-direction: column;
  /* Make the WebView background match the app surface so any pixel not
   * covered by .app-root is dark, not the white default. */
}

.app-root {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 0;
  width: 100%;
  background-color: var(--color-bg-primary);
  border: none;
  transition: var(--transition-base);
  /* Window is now opaque (backgroundColor in tauri.conf.json) so the rounded
   * clip-path trick is unnecessary. The whole content area is rectangular and
   * fills the viewport edge-to-edge — no white border at bottom. */
  overflow: hidden;
}

/* Previously: fullscreen-mode tweak + dark-theme box-shadow on .app-root.
 * Both relied on .app-root having rounded corners / clip-path. With the
 * window now opaque (no clip-path), these tweaks are no-ops and removed. */


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
  animation: modal-appear 0.25s var(--ease-spring-out);
}

@keyframes modal-appear {
  from {
    opacity: 0;
    transform: scale(0.93) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

/* Transient toast — bottom-center, above all content */
.app-toast {
  position: fixed;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  z-index: 20000;
  padding: 9px 16px;
  border-radius: var(--radius-md);
  background-color: var(--color-bg-elevated);
  border: 1px solid var(--color-border-primary);
  box-shadow: var(--shadow-lg);
  font-size: 13px;
  color: var(--color-text-primary);
  max-width: 80vw;
  text-align: center;
  pointer-events: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.2s var(--ease-snappy),
    transform 0.2s var(--ease-snappy);
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px);
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
  /* Disable global dragging feature */
  -webkit-app-region: no-drag;
}

div[role='region'] {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
  /* Ensure these regions are also non-draggable */
  -webkit-app-region: no-drag;
}
</style>
