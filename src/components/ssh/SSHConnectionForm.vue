<template>
  <div
    class="modal-form"
    style="position: relative"
    @keydown.tab="handleTabKey"
  >
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
            ref="nameInput"
            v-model="formData.name"
            type="text"
            :placeholder="$t('ssh.namePlaceholder')"
            class="input"
            :class="{ error: validationErrors.name }"
            required
          />
          <span v-if="validationErrors.name" class="modal-error-message">{{
            validationErrors.name
          }}</span>
        </div>
      </div>

      <div class="modal-form-row">
        <div class="input-container">
          <div class="modal-form-group host-field">
            <label for="host">{{ $t('ssh.host') }} *</label>
            <input
              id="host"
              ref="hostInput"
              v-model="formData.host"
              type="text"
              :placeholder="$t('ssh.hostPlaceholder')"
              class="input"
              :class="{ error: validationErrors.host }"
              required
            />
            <span v-if="validationErrors.host" class="modal-error-message">{{
              validationErrors.host
            }}</span>
          </div>

          <div class="modal-form-group port-field">
            <label for="port">{{ $t('ssh.port') }}</label>
            <input
              id="port"
              ref="portInput"
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
            ref="usernameInput"
            v-model="formData.username"
            type="text"
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
              ref="passwordInput"
              v-model="formData.password"
              :type="showPassword ? 'text' : 'password'"
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
            ref="privateKeyInput"
            v-model="formData.privateKey"
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
              ref="keyPassphraseInput"
              v-model="formData.keyPassphrase"
              :type="showKeyPassphrase ? 'text' : 'password'"
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

      <div class="modal-form-row">
        <div class="modal-form-group">
          <GroupsMultiSelect
            v-model="formData.groups"
            :groups="allGroups"
            :label="$t('ssh.groups')"
            :placeholder="$t('ssh.groupsPlaceholder')"
            :create-group-text="$t('ssh.createGroup')"
            :empty-text="$t('ssh.noGroupsAvailable')"
            @group-added="(group) => allGroups.push(group)"
          />
        </div>

        <div class="modal-form-group">
          <TagsMultiSelect
            v-model="formData.tags"
            :tags="allTags"
            :label="$t('ssh.tags')"
            :placeholder="$t('ssh.tagsPlaceholder')"
            :create-tag-text="$t('ssh.createTag')"
            :empty-text="$t('ssh.noTagsAvailable')"
            @tag-added="(tag) => allTags.push(tag)"
          />
        </div>
      </div>

      <div class="modal-form-row checkbox-row">
        <label class="checkbox-container">
          <input
            ref="saveSessionInput"
            v-model="formData.saveSession"
            type="checkbox"
          />
          <span class="checkbox-label">{{ $t('ssh.saveSession') }}</span>
        </label>
      </div>

      <div class="modal-form-actions">
        <button
          ref="connectButton"
          type="submit"
          class="modal-btn modal-btn-primary"
          :disabled="isLoading"
        >
          {{ isLoading ? $t('ssh.connecting') : $t('ssh.connect') }}
        </button>
        <button
          ref="cancelButton"
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
      :title="connectionErrorTitle"
      :error-message="connectionErrorMessage"
      @close="onCloseProgress"
      @retry="onRetry"
    />
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { Eye, EyeOff } from 'lucide-vue-next';
import ConnectionProgressBar from '../common/ConnectionProgressBar.vue';
import GroupsMultiSelect from '../common/GroupsMultiSelect.vue';
import TagsMultiSelect from '../common/TagsMultiSelect.vue';

type ConnectionStatus = 'connecting' | 'success' | 'error';

interface Props {
  isLoading?: boolean;
  errorMessage?: string | null;
  initialData?: SSHConnectionFormData;
  // Status and progress for ConnectionProgressBar
  showProgress?: boolean;
  connectionStatus?: ConnectionStatus;
  connectionProgress?: number;
  connectionCurrentStep?: number;
  connectionMessage?: string;
  connectionErrorTitle?: string;
  connectionErrorMessage?: string;
}

interface SSHConnectionFormData {
  name: string; // Connection name is now required and at the top
  host: string;
  port: number | null;
  username: string;
  password: string;
  privateKey: string;
  keyPassphrase: string;
  saveSession: boolean;
  groups?: string[]; // Selected group IDs (optional)
  tags?: string[]; // Selected tag IDs (optional)
}

interface ValidationErrors {
  name?: string; // Add name validation error
  host?: string;
  port?: string;
  username?: string;
}

const props = withDefaults(defineProps<Props>(), {
  isLoading: false,
  errorMessage: null,
  initialData: undefined,
  showProgress: false,
  connectionStatus: 'connecting',
  connectionProgress: 0,
  connectionCurrentStep: 0,
  connectionMessage: '',
  connectionErrorTitle: '',
  connectionErrorMessage: '',
});

const formData = reactive<SSHConnectionFormData>({
  name: props.initialData?.name || '',
  host: props.initialData?.host || '',
  port: props.initialData?.port !== undefined && props.initialData?.port !== null ? props.initialData.port : 22,
  username: props.initialData?.username || '',
  password: props.initialData?.password || '',
  privateKey: props.initialData?.privateKey || '',
  keyPassphrase: props.initialData?.keyPassphrase || '',
  saveSession:
    props.initialData?.saveSession !== undefined
      ? props.initialData.saveSession
      : true,
  groups: props.initialData?.groups || [],
  tags: props.initialData?.tags || [],
});

interface Group {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

interface Tag {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

const validationErrors = reactive<ValidationErrors>({});

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  connect: [data: SSHConnectionFormData];
  cancel: [];
  retry: [];
  'close-progress': [];
}>();

// Password visibility state
const showPassword = ref(false);
const showKeyPassphrase = ref(false);

// Groups and Tags state
const allGroups = ref<Group[]>([]);
const allTags = ref<Tag[]>([]);

// Form input references for tab navigation
const nameInput = ref<HTMLInputElement | null>(null);
const hostInput = ref<HTMLInputElement | null>(null);
const portInput = ref<HTMLInputElement | null>(null);
const usernameInput = ref<HTMLInputElement | null>(null);
const passwordInput = ref<HTMLInputElement | null>(null);
const privateKeyInput = ref<HTMLInputElement | null>(null);
const keyPassphraseInput = ref<HTMLInputElement | null>(null);
const saveSessionInput = ref<HTMLInputElement | null>(null);
const connectButton = ref<HTMLElement | null>(null);
const cancelButton = ref<HTMLElement | null>(null);

// Fetch groups and tags on component mount
onMounted(async () => {
  try {
    const groups = await invoke<Group[]>('list_groups');
    allGroups.value = groups || [];
    console.log('Loaded groups:', allGroups.value);
  } catch (error) {
    console.error('Failed to fetch groups:', error);
  }

  try {
    const tags = await invoke<Tag[]>('list_tags');
    allTags.value = tags || [];
    console.log('Loaded tags:', allTags.value);
  } catch (error) {
    console.error('Failed to fetch tags:', error);
  }
});

// Platform-specific UI detection removed; header is now platform-agnostic

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
  if (!formData.name.trim()) {
    validationErrors.name = t('ssh.errorName');
    isValid = false;
  }

  // Validate host address
  if (!formData.host.trim()) {
    validationErrors.host = t('ssh.errorHost');
    isValid = false;
  }

  // Validate port
  if (formData.port !== null && (formData.port < 1 || formData.port > 65535)) {
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

const onSubmit = () => {
  if (!validateForm()) {
    return;
  }

  // 确保 port 有默认值
  const port = formData.port || 22;

  // 构建连接数据
  const submitData: SSHConnectionFormData = {
    ...formData,
    port: port,
  };

  // 移除空的组和标签数组
  if (!submitData.groups || submitData.groups.length === 0) {
    delete submitData.groups;
  }
  if (!submitData.tags || submitData.tags.length === 0) {
    delete submitData.tags;
  }

  // 将会话数据发送给父组件，由后端统一处理保存逻辑
  emit('connect', submitData);
};

const onCancel = () => {
  emit('cancel');
};

const onRetry = () => {
  emit('retry');
};

const onCloseProgress = () => {
  emit('close-progress');
};

// Handle Tab key navigation
const handleTabKey = (event: KeyboardEvent) => {
  // Only handle Tab key
  if (event.key !== 'Tab') return;

  // Get all focusable elements in the form
  const focusableElements = [
    nameInput.value,
    hostInput.value,
    portInput.value,
    usernameInput.value,
    passwordInput.value,
    privateKeyInput.value,
    keyPassphraseInput.value,
    saveSessionInput.value,
    connectButton.value,
    cancelButton.value,
  ].filter(element => element !== null) as HTMLElement[];

  // Find the currently focused element
  const currentIndex = focusableElements.findIndex(
    element => element === document.activeElement
  );

  let nextIndex;

  if (event.shiftKey) {
    // Shift + Tab: move to previous element
    nextIndex =
      currentIndex <= 0 ? focusableElements.length - 1 : currentIndex - 1;
  } else {
    // Tab: move to next element
    nextIndex =
      currentIndex < 0 || currentIndex >= focusableElements.length - 1
        ? 0
        : currentIndex + 1;
  }

  // Focus the next element
  if (focusableElements[nextIndex]) {
    event.preventDefault(); // Prevent default tab behavior
    focusableElements[nextIndex].focus();
  }
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
  box-shadow: 0 0 0 2px rgba(var(--color-primary-rgb), 0.08);
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

.form-general-error {
  width: 100%;
  padding: 8px;
  background-color: rgba(255, 71, 87, 0.1);
  border: 1px solid #ff4757;
  border-radius: var(--radius-sm);
  color: #ff4757;
  font-size: 0.85em;
  margin-bottom: 8px;
  text-align: center;
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
  transition: color 0.2s ease;
}

.checkbox-container:hover .checkbox-label {
  color: var(--color-text-primary);
}
</style>
