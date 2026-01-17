<template>
  <transition name="progress-fade">
    <div v-if="visible" class="connection-progress-overlay">
      <div class="progress-container" :class="{ 'error-state': isError }">
        <!-- Success overlay: large centered check shown when connection succeeds -->
        <div
          v-if="isSuccess && !isError"
          class="success-overlay"
          aria-hidden="true"
        >
          <div class="success-icon-wrapper-centered">
            <component :is="CheckIcon" :size="96" class="success-icon" />
          </div>
        </div>
        <div class="progress-header">
          <h3 class="progress-title">
            <component
              :is="ErrorIcon"
              v-if="isError"
              :size="24"
              class="error-icon"
            />
            <component
              :is="LoadingIcon"
              v-else
              :size="24"
              class="loading-icon"
            />
            {{ headerTitle }}
          </h3>
        </div>

        <div class="progress-content">
          <!-- Success State -->
          <template v-if="isSuccess && !isError">
            <div class="status-steps">
              <div
                v-for="(step, index) in steps"
                :key="index"
                class="status-step"
                :class="{ completed: index < currentStep }"
              >
                <div class="step-indicator">
                  <component
                    :is="CheckIcon"
                    v-if="index < currentStep"
                    :size="16"
                  />
                  <span v-else>{{ index + 1 }}</span>
                </div>
                <span class="step-label">{{ step }}</span>
              </div>
            </div>
            <p class="progress-message">{{ message }}</p>
          </template>

          <!-- Error State -->
          <template v-else-if="isError">
            <!-- Status indicators with error inline -->
            <div class="status-steps">
              <div
                v-for="(step, index) in steps"
                :key="index"
                class="status-step"
                :class="{
                  completed: index < currentStep,
                  failed: index === currentStep,
                  pending: index > currentStep,
                }"
              >
                <div
                  class="step-indicator"
                  :class="{ error: index === currentStep }"
                >
                  <component
                    :is="CheckIcon"
                    v-if="index < currentStep"
                    :size="16"
                  />
                  <component
                    :is="ErrorIcon"
                    v-else-if="index === currentStep"
                    :size="16"
                  />
                  <span v-else>{{ index + 1 }}</span>
                </div>
                <div class="step-content">
                  <span class="step-label">{{ step }}</span>
                  <p
                    v-if="index === currentStep && errorMessage"
                    class="step-error-message"
                  >
                    {{
                      typeof errorMessage === 'string'
                        ? errorMessage
                        : String(errorMessage)
                    }}
                  </p>
                </div>
              </div>
            </div>
          </template>

          <!-- Loading State -->
          <template v-else>
            <!-- Animated loading spinner -->
            <div class="spinner">
              <div class="spinner-ring"></div>
            </div>

            <!-- Progress message -->
            <p class="progress-message">{{ message }}</p>

            <!-- Progress bar -->
            <div class="progress-bar-wrapper">
              <div class="progress-bar">
                <div
                  class="progress-bar-fill"
                  :style="{ width: progress + '%' }"
                ></div>
              </div>
              <span class="progress-percentage">{{ progress }}%</span>
            </div>

            <!-- Status indicators with inline error message -->
            <div class="status-steps">
              <div
                v-for="(step, index) in steps"
                :key="index"
                class="status-step"
                :class="{
                  completed: index < currentStep,
                  active: index === currentStep,
                }"
              >
                <div class="step-indicator">
                  <component
                    :is="CheckIcon"
                    v-if="index < currentStep"
                    :size="16"
                  />
                  <span v-else>{{ index + 1 }}</span>
                </div>
                <div class="step-content">
                  <span class="step-label">{{ step }}</span>
                  <p
                    v-if="index === currentStep && errorMessage"
                    class="step-error-message"
                  >
                    {{
                      typeof errorMessage === 'string'
                        ? errorMessage
                        : String(errorMessage)
                    }}
                  </p>
                </div>
              </div>
            </div>
          </template>

          <!-- Action buttons -->
          <div v-if="isError || isConnecting" class="action-buttons">
            <button v-if="isError" class="btn btn-primary" @click="handleRetry">
              Retry
            </button>
            <button
              class="btn"
              :class="isError ? 'btn-secondary' : 'btn-secondary'"
              @click="handleClose"
            >
              {{ isError ? 'Close' : 'Cancel' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import {
  Check as CheckIcon,
  AlertCircle as ErrorIcon,
  Loader as LoadingIcon,
} from 'lucide-vue-next';

type ConnectionStatus = 'connecting' | 'success' | 'error';

interface Props {
  visible?: boolean;
  status?: ConnectionStatus; // 'connecting' | 'success' | 'error'
  title?: string;
  message?: string;
  errorMessage?: string; // Error details
  progress?: number; // 0-100
  currentStep?: number; // 0-based index of current step
  steps?: string[];
}

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  status: 'connecting',
  title: '',
  message: '',
  errorMessage: '',
  progress: 0,
  currentStep: 0,
  steps: () => [],
});

const emit = defineEmits<{
  retry: [];
  close: [];
}>();

// Compute status states
const isError = computed(() => props.status === 'error');
const isSuccess = computed(() => props.status === 'success');
const isConnecting = computed(() => props.status === 'connecting');

// Default steps (English)
const defaultSteps = computed(() => [
  'Verifying connection',
  'Authenticating user',
  'Initializing terminal',
]);

const steps = computed(() => {
  return props.steps && props.steps.length > 0
    ? props.steps
    : defaultSteps.value;
});

// Header title (English)
const headerTitle = computed(() => {
  if (isError.value) {
    return props.title || 'Connection Failed';
  }
  if (isSuccess.value) {
    return props.title || 'Connection Successful';
  }
  return props.title || 'Connecting...';
});

// Message (English)
const message = computed(() => {
  if (isError.value) {
    return props.message || 'Failed to establish connection';
  }
  if (isSuccess.value) {
    return props.message || 'Connection established successfully';
  }
  return props.message || 'Establishing SSH connection';
});

// Event handlers
const handleRetry = () => {
  emit('retry');
};

const handleClose = () => {
  emit('close');
};
</script>

<style scoped>
.connection-progress-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  border-radius: var(--radius-lg);
}

.progress-container {
  background-color: transparent;
  padding: 16px;
  width: 100%;
  height: 100%;
  box-shadow: none;
  display: flex;
  flex-direction: column;
  position: relative; /* allow absolute children like success overlay */
}

/* Success overlay centered inside the form */

.success-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 10;
  background: rgba(0, 0, 0, 0.06); /* slight dim of background */
}

.success-icon-wrapper-centered {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 140px;
  height: 140px;
  border-radius: 24px;
  background: rgba(255, 255, 255, 0.98);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.16);
  transform: scale(0.85);
  animation: success-pop 260ms cubic-bezier(0.2, 0.9, 0.2, 1) forwards;
}

.success-icon {
  color: var(--color-success, #10b981);
}

@keyframes success-pop {
  from {
    transform: scale(0.8);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

.progress-container {
  background-color: transparent;
  padding: 16px;
  width: 100%;
  height: 100%;
  box-shadow: none;
  display: flex;
  flex-direction: column;
}

@keyframes progress-appear {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.progress-header {
  margin-bottom: 24px;
  text-align: center;
  padding: 8px 0;
  border-bottom: none;
  background: transparent;
  margin: 0 0 24px 0;
}

.progress-title {
  margin: 0;
  font-size: 1.1em;
  font-weight: 600;
  color: var(--color-text-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

.progress-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
  flex: 1;
  justify-content: center;
}

/* Spinner animation */
.spinner {
  display: flex;
  justify-content: center;
  margin: 16px 0;
}

.spinner-ring {
  width: 48px;
  height: 48px;
  border: 3px solid var(--color-border-default);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.spinner-ring.error-spinner {
  border-top-color: #ef4444;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* Message text */
.progress-message {
  margin: 0;
  text-align: center;
  font-size: 14px;
  color: var(--color-text-secondary);
  line-height: 1.5;
}

/* Progress bar */
.progress-bar-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
}

.progress-bar {
  flex: 1;
  height: 6px;
  background-color: var(--color-bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: var(--color-primary);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.progress-percentage {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  min-width: 32px;
  text-align: right;
}

/* Status steps */
.status-steps {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 12px;
  padding: 0 16px;
}

.status-step {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: var(--color-text-secondary);
  transition: all 0.2s ease;
}

.status-step.active {
  color: var(--color-primary);
  font-weight: 500;
}

.status-step.completed {
  color: var(--color-text-secondary);
}

.status-step.pending {
  color: var(--color-text-secondary);
  opacity: 0.6;
}

.step-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background-color: var(--color-bg-tertiary);
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.status-step.active .step-indicator {
  background-color: var(--color-primary);
  color: white;
}

.status-step.completed .step-indicator {
  background-color: var(--color-primary);
  color: white;
}

.step-label {
  flex: 1;
}

.step-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.step-error-message {
  margin: 0;
  font-size: 12px;
  color: #ef4444;
  font-weight: 400;
  line-height: 1.4;
  word-break: break-word;
  padding: 8px 10px;
  border-radius: 4px;
  border-left: 3px solid #ef4444;
  margin-top: 6px;
}

.status-step.failed {
  color: #ef4444;
}

.status-step.failed .step-indicator.error {
  background-color: #ef4444;
  color: white;
}

.error-icon {
  color: #ff4757;
  margin-right: 8px;
  animation: pulse-error 2s ease-in-out infinite;
}

.loading-icon {
  color: var(--color-primary);
  margin-right: 8px;
  animation: spin 1s linear infinite;
}

/* Success state styles */
.success-icon-wrapper {
  display: flex;
  justify-content: center;
  margin: 20px 0;
  color: #2ed573;
  animation: success-bounce 0.6s ease-out;
  opacity: 0;
  pointer-events: none;
}

@keyframes success-bounce {
  0% {
    transform: scale(0);
    opacity: 0;
  }
  70% {
    transform: scale(1.1);
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
}

/* Action buttons */
.action-buttons {
  display: flex;
  gap: 12px;
  margin-top: auto;
  padding-top: 24px;
}

.btn {
  flex: 1;
  padding: 8px 16px;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 0.9em;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: center;
}

.btn-primary {
  background-color: var(--color-primary);
  color: white;
}

.btn-primary:hover {
  filter: brightness(1.1);
  transform: translateY(-1px);
}

.btn-primary:active {
  transform: translateY(0);
}

.btn-secondary {
  background-color: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.btn-secondary:hover {
  background-color: var(--color-bg-secondary);
  color: var(--color-text-primary);
  transform: translateY(-1px);
}

.btn-secondary:active {
  transform: translateY(0);
}

/* Transition animations */
.progress-fade-enter-active,
.progress-fade-leave-active {
  transition: opacity 0.3s ease;
}

.progress-fade-enter-from,
.progress-fade-leave-to {
  opacity: 0;
}
</style>
