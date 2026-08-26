<template>
  <div class="modal-form" style="position: relative">
    <div class="modal-header">
      <h3 class="modal-title">
        {{ $t('ssh.title') }}
      </h3>
    </div>

    <form @submit.prevent="onSubmit">
      <!-- Connection name field moved to the top and marked as required -->
      <div class="modal-form-row">
        <div class="modal-form-group full-width">
          <label for="name">{{ $t('ssh.name') }} *</label>
          <input
            id="name"
            v-model="formData.server_name"
            type="text"
            :placeholder="$t('ssh.namePlaceholder')"
            class="input"
            :class="{ error: validationErrors.server_name }"
            required
          />
          <span
            v-if="validationErrors.server_name"
            class="modal-error-message"
            >{{ validationErrors.server_name }}</span
          >
        </div>
      </div>

      <div class="modal-form-row">
        <div class="input-container">
          <div class="modal-form-group host-field">
            <label for="host">{{ $t('ssh.host') }} *</label>
            <input
              id="host"
              v-model="formData.addr"
              type="text"
              :placeholder="$t('ssh.hostPlaceholder')"
              class="input"
              :class="{ error: validationErrors.addr }"
              required
            />
            <span v-if="validationErrors.addr" class="modal-error-message">{{
              validationErrors.addr
            }}</span>
          </div>

          <div class="modal-form-group port-field">
            <label for="port">{{ $t('ssh.port') }}</label>
            <input
              id="port"
              v-model.number="formData.port"
              type="number"
              min="1"
              max="65535"
              placeholder="22"
              class="input short-input"
              :class="{ error: validationErrors.port }"
            />
            <span v-if="validationErrors.port" class="modal-error-message">{{
              validationErrors.port
            }}</span>
          </div>
        </div>
      </div>

      <div class="modal-form-row">
        <div class="modal-form-group full-width">
          <label for="username">{{ $t('ssh.username') }} *</label>
          <input
            id="username"
            v-model="formData.username"
            type="text"
            autocomplete="off"
            spellcheck="false"
            :placeholder="$t('ssh.usernamePlaceholder')"
            class="input"
            :class="{ error: validationErrors.username }"
            required
          />
          <span v-if="validationErrors.username" class="modal-error-message">{{
            validationErrors.username
          }}</span>
        </div>
      </div>

      <div class="modal-form-row">
        <div class="modal-form-group full-width">
          <label for="password">{{ $t('ssh.password') }}</label>
          <!-- Password field with show/hide toggle -->
          <div class="password-input-container">
            <input
              id="password"
              v-model="formData.password"
              :type="showPassword ? 'text' : 'password'"
              autocomplete="new-password"
              spellcheck="false"
              :placeholder="$t('ssh.passwordPlaceholder')"
              class="input"
            />
            <button
              type="button"
              class="password-toggle-btn"
              :aria-label="
                showPassword ? $t('ssh.hidePassword') : $t('ssh.showPassword')
              "
              @click="togglePasswordVisibility"
            >
              <component
                :is="showPassword ? EyeOff : Eye"
                :size="16"
                :stroke-width="1.5"
              />
            </button>
          </div>
        </div>
      </div>

      <div class="modal-form-row">
        <div class="modal-form-group">
          <label for="privateKey">{{ $t('ssh.privateKey') }}</label>
          <input
            id="privateKey"
            v-model="formData.private_key_path"
            type="text"
            :placeholder="$t('ssh.privateKeyPlaceholder')"
            class="input"
          />
        </div>

        <div class="modal-form-group">
          <label for="keyPassphrase">{{ $t('ssh.passphrase') }}</label>
          <!-- Key passphrase field with show/hide toggle -->
          <div class="password-input-container">
            <input
              id="keyPassphrase"
              v-model="formData.key_passphrase"
              :type="showKeyPassphrase ? 'text' : 'password'"
              autocomplete="new-password"
              spellcheck="false"
              :placeholder="$t('ssh.passphrasePlaceholder')"
              class="input"
            />
            <button
              type="button"
              class="password-toggle-btn"
              :aria-label="
                showKeyPassphrase
                  ? $t('ssh.hideKeyPassphrase')
                  : $t('ssh.showKeyPassphrase')
              "
              @click="toggleKeyPassphraseVisibility"
            >
              <component
                :is="showKeyPassphrase ? EyeOff : Eye"
                :size="16"
                :stroke-width="1.5"
              />
            </button>
          </div>
        </div>
      </div>

      <!-- Only for existing sessions: an explicit way to clear the stored
           password/passphrase instead of silently leaving the ciphertext. -->
      <div v-if="isEditMode" class="modal-form-row checkbox-row">
        <label class="checkbox-container">
          <input
            v-model="clearStoredCredentials"
            type="checkbox"
          />
          <span class="checkbox-label">{{ $t('ssh.clearStoredPassword') }}</span>
        </label>
      </div>

      <div class="modal-form-row tags-row">
        <div class="modal-form-group full-width">
          <GroupsMultiSelect
            v-model="formData.groups"
            :groups="allGroups"
            :label="$t('ssh.groups')"
            :placeholder="$t('ssh.groupsPlaceholder')"
            :create-group-text="$t('ssh.createGroup')"
            :empty-text="$t('ssh.noGroupsAvailable')"
            :immediate-save="true"
            @group-added="handleGroupAdded"
          />
        </div>
      </div>

      <div class="modal-form-row tags-row">
        <div class="modal-form-group full-width">
          <TagsMultiSelect
            v-model="formData.tags"
            :tags="allTags"
            :label="$t('ssh.tags')"
            :placeholder="$t('ssh.tagsPlaceholder')"
            :create-tag-text="$t('ssh.createTag')"
            :empty-text="$t('ssh.noTagsAvailable')"
            :immediate-save="true"
            @tag-added="handleTagAdded"
          />
        </div>
      </div>

      <div class="modal-form-row checkbox-row">
        <label class="checkbox-container">
          <input
            v-model="formData.save_session"
            type="checkbox"
          />
          <span class="checkbox-label">{{ $t('ssh.saveSession') }}</span>
        </label>
      </div>

      <div class="modal-form-actions">
        <button
          type="submit"
          class="modal-btn modal-btn-primary"
          :disabled="isLoading"
        >
          {{ isLoading ? $t('ssh.connecting') : $t('ssh.connect') }}
        </button>
        <button
          type="button"
          class="modal-btn modal-btn-secondary"
          :disabled="isLoading"
          @click="onSaveOnly"
        >
          {{ $t('ssh.saveOnly') }}
        </button>
        <button
          type="button"
          class="modal-btn modal-btn-secondary"
          :disabled="isLoading"
          @click="onCancel"
        >
          {{ $t('ssh.cancel') }}
        </button>
      </div>
    </form>

    <ConnectionProgressBar
      :visible="showProgress"
      :status="connectionStatus"
      :progress="connectionProgress"
      :current-step="connectionCurrentStep"
      :message="connectionMessage"
      :time="connectionTime"
      :title="connectionErrorTitle"
      :error-message="connectionErrorMessage"
      @close="onCloseProgress"
      @retry="onRetry"
    />
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { Eye, EyeOff } from 'lucide-vue-next';
import ConnectionProgressBar from './ConnectionProgressBar.vue';
import GroupsMultiSelect from '../common/GroupsMultiSelect.vue';
import TagsMultiSelect from '../common/TagsMultiSelect.vue';
import type { MetadataItem } from '@/core/types/common';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';
import { createLogger } from '@/core/utils/logger';
import type { SSHConnectionFormData } from '@/features/session/types';

const logger = createLogger('SSHConnectionForm');

type ConnectionStatus = 'connecting' | 'success' | 'error';

interface Props {
  isLoading?: boolean;
  initialData?: SSHConnectionFormData;
  // Status and progress for ConnectionProgressBar
  showProgress?: boolean;
  connectionStatus?: ConnectionStatus;
  connectionProgress?: number;
  connectionCurrentStep?: number;
  connectionMessage?: string;
  connectionTime?: number;
  connectionErrorTitle?: string;
  connectionErrorMessage?: string;
}

interface ValidationErrors {
  server_name?: string;
  addr?: string;
  port?: string;
  username?: string;
}

const props = withDefaults(defineProps<Props>(), {
  isLoading: false,
  initialData: undefined,
  showProgress: false,
  connectionStatus: 'connecting',
  connectionProgress: 0,
  connectionCurrentStep: 0,
  connectionMessage: '',
  connectionTime: 0,
  connectionErrorTitle: '',
  connectionErrorMessage: '',
});

const formData = reactive<SSHConnectionFormData>({
  id: props.initialData?.id || undefined,
  server_name: props.initialData?.server_name || '',
  addr: props.initialData?.addr || '',
  port:
    props.initialData?.port !== undefined && props.initialData?.port !== null
      ? props.initialData.port
      : 22,
  username: props.initialData?.username || '',
  password: props.initialData?.password || '',
  private_key_path: props.initialData?.private_key_path || '',
  key_passphrase: props.initialData?.key_passphrase || '',
  save_session:
    props.initialData?.save_session !== undefined
      ? props.initialData.save_session
      : true,
  groups: props.initialData?.groups || [],
  tags: props.initialData?.tags || [],
});

const validationErrors = reactive<ValidationErrors>({});

// Watch for initialData updates (useful for background credential loading or re-editing).
  // The parent always replaces the whole object (never mutates it in place),
  // so a shallow reference watch suffices — no deep:true needed.
  watch(
    () => props.initialData,
    newData => {
      if (newData) {
        if (newData.id !== undefined) formData.id = newData.id;
        // Update basic fields if they are different
        if (newData.server_name !== undefined)
          formData.server_name = newData.server_name;
        if (newData.addr !== undefined) formData.addr = newData.addr;
        if (newData.port !== undefined) formData.port = newData.port;
        if (newData.username !== undefined)
          formData.username = newData.username;
        if (newData.private_key_path !== undefined)
          formData.private_key_path = newData.private_key_path;
        if (newData.save_session !== undefined)
          formData.save_session = newData.save_session;
        if (newData.groups !== undefined) formData.groups = [...newData.groups];
        if (newData.tags !== undefined) formData.tags = [...newData.tags];

        // Update sensitive fields if provided
        if (newData.password) {
          formData.password = newData.password;
        }
        if (newData.key_passphrase) {
          formData.key_passphrase = newData.key_passphrase;
        }
      }
    },
    { immediate: true }
  );

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  connect: [data: SSHConnectionFormData];
  save: [data: SSHConnectionFormData];
  cancel: [];
  retry: [];
  'close-progress': [];
}>();

// Password visibility state
const showPassword = ref(false);
const showKeyPassphrase = ref(false);

// Whether this form is editing an existing saved session (has an id). In edit
// mode empty secret fields are omitted from submissions so stored credentials
// are preserved (see buildSubmitData), and the clear toggle explicitly nulls
// them so the backend can drop the stored ciphertext.
const isEditMode = computed(() => !!props.initialData?.id);
const clearStoredCredentials = ref(false);

// Groups and Tags state
const allGroups = ref<MetadataItem[]>([]);
const allTags = ref<MetadataItem[]>([]);
const newlyCreatedGroups = ref<string[]>([]);
const newlyCreatedTags = ref<string[]>([]);

// Fetch groups and tags on component mount
onMounted(async () => {
  eventBus.on(APP_EVENTS.CLOSE_DIALOG, onCancel);

  try {
    const groups = await invoke<MetadataItem[]>('list_groups');
    allGroups.value = groups || [];
  } catch (error) {
    logger.error('Failed to fetch groups', error);
  }

  try {
    const tags = await invoke<MetadataItem[]>('list_tags');
    allTags.value = tags || [];
  } catch (error) {
    logger.error('Failed to fetch tags', error);
  }
});

onUnmounted(() => {
  eventBus.off(APP_EVENTS.CLOSE_DIALOG, onCancel);
});

// Platform-specific UI detection removed; header is now platform-agnostic

const handleGroupAdded = (group: MetadataItem) => {
  allGroups.value.push(group);
  newlyCreatedGroups.value.push(group.id);
};

const handleTagAdded = (tag: MetadataItem) => {
  allTags.value.push(tag);
  newlyCreatedTags.value.push(tag.id);
};

// Toggle password visibility
const togglePasswordVisibility = () => {
  showPassword.value = !showPassword.value;
};

// Toggle key passphrase visibility
const toggleKeyPassphraseVisibility = () => {
  showKeyPassphrase.value = !showKeyPassphrase.value;
};

const validateForm = (): boolean => {
  // Clear previous errors
  Object.keys(validationErrors).forEach(key => {
    delete validationErrors[key as keyof ValidationErrors];
  });

  let isValid = true;

  // Validate connection name
  if (!formData.server_name.trim()) {
    validationErrors.server_name = t('ssh.errorName');
    isValid = false;
  }

  // Validate host address
  if (!formData.addr.trim()) {
    validationErrors.addr = t('ssh.errorHost');
    isValid = false;
  }

  // Normalize an empty port ('' from the number input) to null first, so a
  // blank port is treated as "use the default 22" rather than a validation
  // error from comparing '' against the 1-65535 range.
  const port = normalizePort(formData.port);
  if (port !== null && (port < 1 || port > 65535)) {
    validationErrors.port = t('ssh.errorPort');
    isValid = false;
  }

  // Validate username
  if (!formData.username.trim()) {
    validationErrors.username = t('ssh.errorUsername');
    isValid = false;
  }

  return isValid;
};

/** Collapse an empty/invalid port into `null` so the caller falls back to 22. */
const normalizePort = (port: unknown): number | null => {
  if (port === undefined || port === null || port === '') return null;
  const num = typeof port === 'number' ? port : Number(port);
  return Number.isFinite(num) ? num : null;
};

/**
 * Build the payload shared by Connect and Save-Only, normalizing the port and
 * dropping empty groups/tags. In edit mode, empty secret fields are stripped
 * (not sent as empty strings) so the backend keeps the stored ciphertext,
 * unless the user explicitly asked to clear them — then `null` is sent.
 */
const buildSubmitData = (): SSHConnectionFormData => {
  const data: SSHConnectionFormData = {
    ...formData,
    port: normalizePort(formData.port) ?? 22,
  };

  // Remove empty groups and tags arrays
  if (!data.groups || data.groups.length === 0) {
    delete data.groups;
  }
  if (!data.tags || data.tags.length === 0) {
    delete data.tags;
  }

  // Edit mode: don't clobber stored credentials with empty strings. If the
  // user typed nothing, omit the field so the backend leaves the ciphertext
  // untouched; only an explicit "clear" sends nulls to clear them.
  if (isEditMode.value) {
    if (clearStoredCredentials.value) {
      data.password = null;
    } else if (!data.password) {
      delete data.password;
    }
    if (clearStoredCredentials.value) {
      data.key_passphrase = null;
    } else if (!data.key_passphrase) {
      delete data.key_passphrase;
    }
    data.clearCredentials = clearStoredCredentials.value;
  }

  return data;
};

/**
 * After a successful save/connect, the groups/tags that were created live
 * through this form are persisted, so they must NOT be rolled back later if
 * the user cancels the (already succeeded) flow.
 */
const markMetadataCommitted = () => {
  newlyCreatedGroups.value = [];
  newlyCreatedTags.value = [];
};

const onSubmit = () => {
  if (!validateForm()) {
    return;
  }
  markMetadataCommitted();
  // Send session data to parent component for unified saving/processing
  emit('connect', buildSubmitData());
};

const onSaveOnly = () => {
  if (!validateForm()) {
    return;
  }
  markMetadataCommitted();
  // Send session data to parent component for only save logic
  emit('save', buildSubmitData());
};

const onCancel = async () => {
  // If connection was successful, we don't rollback
  if (props.connectionStatus === 'success') {
    emit('cancel');
    return;
  }

  // Rollback newly created groups and tags if cancelled
  if (newlyCreatedGroups.value.length > 0) {
    for (const id of newlyCreatedGroups.value) {
      try {
        await invoke('delete_group', { id });
      } catch (error) {
        logger.error('Failed to rollback group', { id, error });
      }
    }
    eventBus.emit(APP_EVENTS.GROUPS_UPDATED);
    newlyCreatedGroups.value = [];
  }

  if (newlyCreatedTags.value.length > 0) {
    for (const id of newlyCreatedTags.value) {
      try {
        await invoke('delete_tag', { id });
      } catch (error) {
        logger.error('Failed to rollback tag', { id, error });
      }
    }
    eventBus.emit(APP_EVENTS.TAGS_UPDATED);
    newlyCreatedTags.value = [];
  }

  emit('cancel');
};

const onRetry = () => {
  emit('retry');
};

const onCloseProgress = () => {
  emit('close-progress');
};
</script>

<style scoped>
/* Center the header title and align vertically */
.modal-header {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 8px 0;
}

/* Container for host and port inputs */
.input-container {
  display: flex;
  gap: 12px;
  width: 100%;
}

.host-field {
  flex: 2; /* Takes 2/3 of the available space */
}

.port-field {
  flex: 1; /* Takes 1/3 of the available space */
}

/* Password input container */
.password-input-container {
  position: relative;
  display: flex;
  align-items: center;
}

.password-input-container input {
  flex: 1;
  padding-right: 30px; /* Space for the eye icon */
}

.password-toggle-btn {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  z-index: 1; /* Make sure it's above the input */
}

.password-toggle-btn:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-secondary);
}

.tags-row {
  margin-top: -4px;
  margin-bottom: 8px;
}

/* Form layout styles - inheriting from common.css standards */
.modal-form-group.host-field {
  flex: 1 70%;
}

.modal-form-group.port-field {
  flex: 1 30%;
}

/* Standard input field styling - aligned with common.css .modal-input */
.input {
  width: 100%;
  padding: 6px 8px;
  height: 34px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background: var(--color-bg-primary);
  color: var(--color-text-primary);
  font-size: 0.9em;
  box-sizing: border-box;
  transition: all var(--transition-fast);
}

.input:hover {
  border-color: var(--color-border-primary);
}

.input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}

.input::placeholder {
  color: var(--color-text-placeholder);
}

.input.error {
  border-color: #ff4757;
}

.input.error:focus {
  box-shadow: 0 0 0 2px rgba(255, 71, 87, 0.08);
}

/* Override modal-form overflow to allow dropdown menus to show properly */
.modal-form {
  overflow: visible;
}

/* Style error messages consistently */
.modal-error-message {
  display: block;
  color: #ff4757;
  font-size: 0.75em;
  margin-top: 2px;
  margin-bottom: 4px;
}

/* Style actions consistently */
.modal-form-actions {
  display: flex;
  flex-direction: row;
  gap: 8px;
  justify-content: flex-end;
  align-items: center;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--color-border-secondary);
}

/* Button styles for action buttons */
.modal-btn {
  padding: 6px 16px;
  height: 34px;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 0.9em;
  font-weight: 500;
  transition: var(--transition-fast);
  box-sizing: border-box;
  white-space: nowrap;
  user-select: none;
}

.modal-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Specific style for short input fields like port */
.short-input {
  width: 100%; /* Full width within its container */
}

/* Checkbox specific styles */
.checkbox-row {
  display: flex;
  align-items: center;
  margin-top: 4px;
  margin-bottom: 4px;
  padding: 0 2px;
}

.checkbox-container {
  display: flex;
  align-items: center;
  cursor: pointer;
  user-select: none;
  font-size: 0.85em;
  color: var(--color-text-secondary);
}

.checkbox-container input {
  margin-right: 8px;
  cursor: pointer;
  width: auto;
  accent-color: var(--color-primary);
}

.checkbox-label {
  transition: color var(--transition-fast);
}

.checkbox-container:hover .checkbox-label {
  color: var(--color-text-primary);
}
</style>
