<template>
  <div class="modal-form" @keydown.tab="handleTabKey">
    <div class="modal-header">
      <h3 class="modal-title">New SSH Connection</h3>
    </div>

    <form @submit.prevent="onSubmit">
      <!-- Connection name field moved to the top and marked as required -->
      <div class="modal-form-row">
        <div class="modal-form-group full-width">
          <label for="name">Connection Name *</label>
          <input
            id="name"
            ref="nameInput"
            v-model="formData.name"
            type="text"
            placeholder="Name the connection"
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
            <label for="host">Host Address *</label>
            <input
              id="host"
              ref="hostInput"
              v-model="formData.host"
              type="text"
              placeholder="e.g., example.com or hostname"
              class="input"
              :class="{ error: validationErrors.host }"
              required
            />
            <span v-if="validationErrors.host" class="modal-error-message">{{
              validationErrors.host
            }}</span>
          </div>

          <div class="modal-form-group port-field">
            <label for="port">Port</label>
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
          <label for="username">Username *</label>
          <input
            id="username"
            ref="usernameInput"
            v-model="formData.username"
            type="text"
            placeholder="Username"
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
          <label for="password">Password</label>
          <!-- Password field with show/hide toggle -->
          <div class="password-input-container">
            <input
              id="password"
              ref="passwordInput"
              v-model="formData.password"
              :type="showPassword ? 'text' : 'password'"
              placeholder="Password (leave empty for key authentication)"
              class="input"
            />
            <button
              type="button"
              class="password-toggle-btn"
              :aria-label="showPassword ? 'Hide password' : 'Show password'"
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
          <label for="privateKey">Private Key File</label>
          <input
            id="privateKey"
            ref="privateKeyInput"
            v-model="formData.privateKey"
            type="text"
            placeholder="Private key file path"
            class="input"
          />
        </div>

        <div class="modal-form-group">
          <label for="keyPassphrase">Key Passphrase</label>
          <!-- Key passphrase field with show/hide toggle -->
          <div class="password-input-container">
            <input
              id="keyPassphrase"
              ref="keyPassphraseInput"
              v-model="formData.keyPassphrase"
              :type="showKeyPassphrase ? 'text' : 'password'"
              placeholder="Private key passphrase"
              class="input"
            />
            <button
              type="button"
              class="password-toggle-btn"
              :aria-label="
                showKeyPassphrase
                  ? 'Hide key passphrase'
                  : 'Show key passphrase'
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

      <div class="modal-form-actions">
        <div v-if="errorMessage" class="form-general-error">
          {{ errorMessage }}
        </div>
        <button
          ref="connectButton"
          type="submit"
          class="modal-btn modal-btn-primary"
          :disabled="isLoading"
        >
          {{ isLoading ? 'Connecting...' : 'Connect' }}
        </button>
        <button
          ref="cancelButton"
          type="button"
          class="modal-btn modal-btn-secondary"
          :disabled="isLoading"
          @click="onCancel"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { Eye, EyeOff } from 'lucide-vue-next';

interface Props {
  isLoading?: boolean;
  errorMessage?: string | null;
}

defineProps<Props>();

interface SSHConnectionFormData {
  name: string; // Connection name is now required and at the top
  host: string;
  port: number | null;
  username: string;
  password: string;
  privateKey: string;
  keyPassphrase: string;
}

interface ValidationErrors {
  name?: string; // Add name validation error
  host?: string;
  port?: string;
  username?: string;
}

const formData = reactive<SSHConnectionFormData>({
  name: '', // Initialize with empty string
  host: '',
  port: 22,
  username: '',
  password: '',
  privateKey: '',
  keyPassphrase: '',
});

const validationErrors = reactive<ValidationErrors>({});

const emit = defineEmits<{
  connect: [data: SSHConnectionFormData];
  cancel: [];
}>();

// Password visibility state
const showPassword = ref(false);
const showKeyPassphrase = ref(false);

// Form input references for tab navigation
const nameInput = ref<HTMLInputElement | null>(null);
const hostInput = ref<HTMLInputElement | null>(null);
const portInput = ref<HTMLInputElement | null>(null);
const usernameInput = ref<HTMLInputElement | null>(null);
const passwordInput = ref<HTMLInputElement | null>(null);
const privateKeyInput = ref<HTMLInputElement | null>(null);
const keyPassphraseInput = ref<HTMLInputElement | null>(null);
const connectButton = ref<HTMLElement | null>(null);
const cancelButton = ref<HTMLElement | null>(null);

// Platform-specific UI detection removed; header is now platform-agnostic

// Toggle password visibility
const togglePasswordVisibility = () => {
  showPassword.value = !showPassword.value;
};

// Toggle key passphrase visibility
const toggleKeyPassphraseVisibility = () => {
  showKeyPassphrase.value = !showKeyPassphrase.value;
};

// Validate form fields before submission
const validateForm = (): boolean => {
  // Clear previous errors
  Object.keys(validationErrors).forEach(key => {
    delete validationErrors[key as keyof ValidationErrors];
  });

  let isValid = true;

  // Validate connection name
  if (!formData.name.trim()) {
    validationErrors.name = 'Connection name cannot be empty';
    isValid = false;
  }

  // Validate host address
  if (!formData.host.trim()) {
    validationErrors.host = 'Host address cannot be empty';
    isValid = false;
  }

  // Validate port
  if (formData.port !== null && (formData.port < 1 || formData.port > 65535)) {
    validationErrors.port = 'Port number must be between 1-65535';
    isValid = false;
  }

  // Validate username
  if (!formData.username.trim()) {
    validationErrors.username = 'Username cannot be empty';
    isValid = false;
  }

  return isValid;
};

const onSubmit = () => {
  if (!validateForm()) {
    return;
  }

  // Use default port 22 if not specified
  const submitData = {
    ...formData,
    port: formData.port || 22,
  };

  emit('connect', submitData);
};

const onCancel = () => {
  emit('cancel');
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

/* Enhance form group spacing for better visual hierarchy */
.modal-form-group {
  margin-bottom: 8px; /* Consistent spacing between form groups */
}

/* Style labels consistently */
.modal-form-group label {
  display: block;
  margin-bottom: 4px;
  color: var(--color-text-primary);
  font-size: 0.9em;
  font-weight: 500;
}

/* Adjust form row spacing */
.modal-form-row {
  margin-bottom: 10px; /* Reduced from 12px by 2px to meet compactness requirement */
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
  margin-top: 6px; /* Reduced from 8px by 2px to meet compactness requirement */
  padding-top: 6px; /* Reduced from 8px by 2px to meet compactness requirement */
  gap: 6px; /* Reduced from 8px by 2px to meet compactness requirement */
  flex-direction: column;
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
</style>
