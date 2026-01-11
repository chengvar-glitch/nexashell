<template>
  <div class="modal-form" @keydown.tab="handleTabKey">
    <div class="modal-header">
      <div v-if="isMacOS" class="modal-header-left">
        <button class="modal-close-btn modal-macos-close" @click="onCancel">
          <span class="close-icon">●</span>
        </button>
      </div>
      <h3 class="modal-title">New SSH Connection</h3>
      <div v-if="!isMacOS" class="modal-header-right">
        <button class="modal-close-btn modal-windows-close" @click="onCancel">
          <span class="close-icon">×</span>
        </button>
      </div>
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
            :class="{ error: validationErrors.name }"
            required
          />
          <span v-if="validationErrors.name" class="modal-error-message">{{
            validationErrors.name
          }}</span>
        </div>
      </div>

      <div class="modal-form-row">
        <div class="modal-form-group">
          <label for="host">Host Address *</label>
          <!-- Host Address Input Field with enhanced UX -->
          <div
            class="modal-ip-input-container"
            :class="{ error: validationErrors.host }"
          >
            <input
              id="host"
              ref="hostInput"
              v-model="formData.host"
              type="text"
              placeholder="e.g., example.com or hostname"
              :class="{ error: validationErrors.host }"
              required
              @blur="validateHostOnBlur"
            />
            <div
              v-if="showIPSuggestions && ipSuggestions.length > 0"
              class="modal-ip-suggestions"
            >
              <div
                v-for="suggestion in ipSuggestions"
                :key="suggestion"
                class="modal-ip-suggestion-item"
                @click="selectIPSuggestion(suggestion)"
              >
                {{ suggestion }}
              </div>
            </div>
          </div>
          <span v-if="validationErrors.host" class="modal-error-message">{{
            validationErrors.host
          }}</span>
        </div>

        <div class="modal-form-group">
          <label for="port">Port</label>
          <input
            id="port"
            ref="portInput"
            v-model.number="formData.port"
            type="number"
            min="1"
            max="65535"
            placeholder="22"
            :class="{ error: validationErrors.port }"
          />
          <span v-if="validationErrors.port" class="modal-error-message">{{
            validationErrors.port
          }}</span>
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
          <div class="modal-password-input-container">
            <input
              id="password"
              ref="passwordInput"
              v-model="formData.password"
              :type="showPassword ? 'text' : 'password'"
              placeholder="Password (leave empty for key authentication)"
              class="modal-input"
            />
            <button
              type="button"
              class="modal-password-toggle-btn"
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
            class="modal-input"
          />
        </div>

        <div class="modal-form-group">
          <label for="keyPassphrase">Key Passphrase</label>
          <!-- Key passphrase field with show/hide toggle -->
          <div class="modal-password-input-container">
            <input
              id="keyPassphrase"
              ref="keyPassphraseInput"
              v-model="formData.keyPassphrase"
              :type="showKeyPassphrase ? 'text' : 'password'"
              placeholder="Private key passphrase (optional)"
              class="modal-input"
            />
            <button
              type="button"
              class="modal-password-toggle-btn"
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
        <button
          ref="connectButton"
          type="submit"
          class="modal-btn modal-btn-primary"
        >
          Connect
        </button>
        <button
          ref="cancelButton"
          type="button"
          class="modal-btn modal-btn-secondary"
          @click="onCancel"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { Eye, EyeOff } from 'lucide-vue-next';
import { isMacOSBrowser } from '@/utils/app-utils';

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

// Detect if it's a macOS system
const isMacOS = ref(false);

// Password visibility state
const showPassword = ref(false);
const showKeyPassphrase = ref(false);

// IP suggestions related variables
const showIPSuggestions = ref(false);
const ipSuggestions = ref<string[]>([]);

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

onMounted(async () => {
  try {
    isMacOS.value = await isMacOSBrowser();
  } catch (error) {
    console.error('Failed to detect platform:', error);
    isMacOS.value = false;
  }
});

// Toggle password visibility
const togglePasswordVisibility = () => {
  showPassword.value = !showPassword.value;
};

// Toggle key passphrase visibility
const toggleKeyPassphraseVisibility = () => {
  showKeyPassphrase.value = !showKeyPassphrase.value;
};

// Domain validation function
const isValidDomain = (domain: string): boolean => {
  const domainRegex =
    /^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$/;
  return domainRegex.test(domain);
};

// Validate host on blur to provide immediate feedback
const validateHostOnBlur = () => {
  if (formData.host.trim()) {
    if (!isValidDomain(formData.host)) {
      validationErrors.host = 'Please enter a valid domain name';
    } else {
      delete validationErrors.host; // Clear error if valid
    }
  }
};

// Select a suggestion
const selectIPSuggestion = (suggestion: string) => {
  formData.host = suggestion;
  showIPSuggestions.value = false;
  validateHostOnBlur(); // Re-validate after selection
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
  } else if (!isValidDomain(formData.host)) {
    validationErrors.host = 'Please enter a valid domain name';
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
  // Clear any pending suggestions when cancelling
  showIPSuggestions.value = false;
  ipSuggestions.value = [];
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
/* Keep SSH form specific styles that override global styles */
.form-group input {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background: var(--color-bg-primary);
  color: var(--color-text-primary);
  font-size: 0.9em;
  box-sizing: border-box;
}

.form-group input.error {
  border-color: #ff4757;
}

.form-group input:focus {
  outline: none;
  border-color: var(--color-primary);
}

/* Ensure lucide icons are properly styled in password toggle buttons */
.modal-password-toggle-btn svg {
  vertical-align: middle;
}
</style>
