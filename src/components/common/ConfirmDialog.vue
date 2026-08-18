<template>
  <div v-if="visible" class="confirm-dialog-overlay" @click.self="onCancel">
    <div
      ref="dialogRef"
      class="confirm-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      aria-describedby="dialog-message"
      @keydown="handleKeydown"
    >
      <div class="dialog-header">
        <div class="dialog-title-container">
          <AlertCircle
            v-if="isDanger"
            class="dialog-title-icon dialog-title-icon-danger"
          />
          <h3 id="dialog-title" class="dialog-title">{{ resolveTitle }}</h3>
        </div>
      </div>

      <div class="dialog-body">
        <p id="dialog-message" class="dialog-message">{{ message }}</p>
      </div>

      <div class="dialog-footer">
        <button
          ref="cancelBtnRef"
          class="dialog-btn dialog-btn-secondary"
          @click="onCancel"
        >
          {{ resolveCancelText }}
        </button>
        <button
          class="dialog-btn dialog-btn-primary"
          :class="{ 'dialog-btn-danger': isDanger }"
          @click="onConfirm"
        >
          {{ resolveConfirmText }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { AlertCircle } from 'lucide-vue-next';

const { t } = useI18n();

const props = defineProps({
  visible: {
    type: Boolean,
    default: false,
  },
  title: {
    type: String,
    default: '',
  },
  message: {
    type: String,
    default: '',
  },
  confirmText: {
    type: String,
    default: '',
  },
  cancelText: {
    type: String,
    default: '',
  },
  isDanger: {
    type: Boolean,
    default: false,
  },
});

// Localized fallbacks when the caller does not provide explicit text.
const resolveTitle = computed(() => props.title || t('common.confirmTitle'));
const resolveConfirmText = computed(
  () => props.confirmText || t('common.confirm')
);
const resolveCancelText = computed(
  () => props.cancelText || t('common.cancel')
);

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();

const dialogRef = ref<HTMLElement | null>(null);
const cancelBtnRef = ref<HTMLElement | null>(null);
let lastFocused: HTMLElement | null = null;

// Store the previously focused element on open so focus can be restored on
// close; focus the cancel button on open (safe default for a destructive
// action — never auto-focus the confirm/danger button).
watch(
  () => props.visible,
  visible => {
    if (visible) {
      lastFocused = document.activeElement as HTMLElement | null;
      nextTick(() => {
        cancelBtnRef.value?.focus();
      });
    }
  }
);

const restoreFocus = () => {
  if (lastFocused && document.contains(lastFocused)) lastFocused.focus();
  lastFocused = null;
};

const onConfirm = () => {
  emit('confirm');
  restoreFocus();
};

const onCancel = () => {
  emit('cancel');
  restoreFocus();
};

// Keyboard: Escape cancels; Tab cycles focus within the dialog (focus trap).
const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    e.preventDefault();
    onCancel();
    return;
  }
  if (e.key !== 'Tab' || !dialogRef.value) return;
  const focusables = dialogRef.value.querySelectorAll<HTMLElement>(
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
  );
  if (focusables.length === 0) return;
  const first = focusables[0];
  const last = focusables[focusables.length - 1];
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault();
    first.focus();
  }
};
</script>

<style scoped>
.confirm-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--color-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: overlay-fade-in var(--transition-fast);
}

@keyframes overlay-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.confirm-dialog {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
  min-width: 320px;
  max-width: 420px;
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  animation: dialog-scale-in var(--transition-fast)
    cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes dialog-scale-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.dialog-header {
  padding: 20px 20px 12px 20px;
  border-bottom: 1px solid var(--color-border-secondary);
}

.dialog-title-container {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dialog-title-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  color: #ff3b30;
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .dialog-title-icon-danger {
    color: #ff453a;
  }
}

:root.theme-dark .dialog-title-icon-danger {
  color: #ff453a;
}

.dialog-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: -0.3px;
}

.dialog-body {
  padding: 12px 20px;
}

.dialog-message {
  margin: 0;
  font-size: 14px;
  color: var(--color-text-secondary);
  line-height: 1.6;
  letter-spacing: -0.2px;
}

.dialog-footer {
  padding: 16px 20px;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  border-top: 1px solid var(--color-border-secondary);
  background: var(--color-bg-secondary);
}

.dialog-btn {
  padding: 8px 16px;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
  min-width: 76px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  letter-spacing: -0.2px;
  outline: none;
}

.dialog-btn:hover {
  transform: translateY(-1px);
}

.dialog-btn:active {
  transform: translateY(0);
}

.dialog-btn:focus-visible {
  box-shadow: var(--focus-ring);
}

.dialog-btn-secondary {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-secondary);
}

.dialog-btn-secondary:hover {
  background: var(--color-border-primary);
  transition: all var(--transition-fast);
}

.dialog-btn-secondary:active {
  background: var(--color-bg-secondary);
  opacity: 0.9;
}

.dialog-btn-primary {
  background: var(--color-primary);
  color: white;
  border: none;
  font-weight: 600;
}

.dialog-btn-primary:hover {
  background: var(--color-primary-hover);
  transition: all var(--transition-fast);
}

.dialog-btn-primary:active {
  background: var(--color-primary);
  opacity: 0.85;
}

.dialog-btn-danger {
  background: #ff3b30;
  color: white;
}

.dialog-btn-danger:hover {
  background: #ff2d55;
  transition: all var(--transition-fast);
}

.dialog-btn-danger:active {
  background: #ff3b30;
  opacity: 0.85;
}

/* Dark theme support */
@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .dialog-btn-danger {
    background: #ff453a;
  }

  :root:not(.theme-light) .dialog-btn-danger:hover {
    background: #ff6b5b;
  }

  :root:not(.theme-light) .dialog-btn-danger:active {
    background: #ff453a;
    opacity: 0.85;
  }
}

:root.theme-dark .dialog-btn-danger {
  background: #ff453a;
}

:root.theme-dark .dialog-btn-danger:hover {
  background: #ff6b5b;
}

:root.theme-dark .dialog-btn-danger:active {
  background: #ff453a;
  opacity: 0.85;
}

/* Disabled state */
.dialog-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}
</style>
