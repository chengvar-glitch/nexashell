<template>
  <div v-if="visible" class="export-overlay" @click.self="handleCancel">
    <div class="export-dialog" role="dialog" aria-modal="true" :aria-label="t('home.exportTitle')">
      <div class="export-header">
        <h3 class="export-title">{{ t('home.exportTitle') }}</h3>
        <button
          class="export-close"
          :disabled="busy"
          :title="t('common.cancel')"
          @click="handleCancel"
        >
          <X :size="16" />
        </button>
      </div>

      <p class="export-subtitle">{{ t('home.exportHint', { count }) }}</p>

      <div class="export-field">
        <label class="export-label" for="export-password">
          {{ t('home.exportPassword') }}
        </label>
        <input
          id="export-password"
          ref="passwordRef"
          v-model="password"
          type="password"
          class="export-input"
          autocomplete="new-password"
          :placeholder="t('home.exportPasswordPlaceholder')"
          :disabled="busy"
          @keydown.esc="handleCancel"
          @keydown.enter="passwordRefConfirm?.focus()"
        />
      </div>

      <div class="export-field">
        <label class="export-label" for="export-password-confirm">
          {{ t('home.exportPasswordConfirm') }}
        </label>
        <input
          id="export-password-confirm"
          ref="passwordRefConfirm"
          v-model="passwordConfirm"
          type="password"
          class="export-input"
          autocomplete="new-password"
          :placeholder="t('home.exportPasswordPlaceholder')"
          :disabled="busy"
          @keydown.esc="handleCancel"
          @keydown.enter="handleConfirm"
        />
      </div>

      <div v-if="errorMessage" class="export-error" role="alert">
        {{ errorMessage }}
      </div>

      <div class="export-footer">
        <button class="export-btn secondary" :disabled="busy" @click="handleCancel">
          {{ t('common.cancel') }}
        </button>
        <button
          class="export-btn primary"
          :disabled="busy || !password || !passwordConfirm"
          @click="handleConfirm"
        >
          <Loader2 v-if="busy" :size="14" class="spin" />
          {{ busy ? t('home.exporting') : t('home.exportConfirm') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Loader2, X } from 'lucide-vue-next';

const props = defineProps<{
  visible: boolean;
  count: number;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  /** User confirmed; parent performs the actual file save + export. */
  confirm: [password: string];
}>();

const { t } = useI18n();

const passwordRef = ref<HTMLInputElement | null>(null);
const passwordRefConfirm = ref<HTMLInputElement | null>(null);
const password = ref('');
const passwordConfirm = ref('');
const busy = ref(false);
const errorMessage = ref('');

watch(
  () => props.visible,
  async value => {
    if (!value) return;
    password.value = '';
    passwordConfirm.value = '';
    errorMessage.value = '';
    busy.value = false;
    await nextTick();
    passwordRef.value?.focus();
  }
);

const handleConfirm = () => {
  if (busy.value) return;
  if (!password.value || !passwordConfirm.value) {
    errorMessage.value = t('home.passwordRequired');
    return;
  }
  if (password.value !== passwordConfirm.value) {
    errorMessage.value = t('home.passwordMismatch');
    return;
  }
  busy.value = true;
  emit('confirm', password.value);
};

const handleCancel = () => {
  if (busy.value) return;
  emit('update:visible', false);
};

/** Parent calls this after the export settles (success or failure). */
const finish = () => {
  busy.value = false;
  emit('update:visible', false);
};

defineExpose({ finish });
</script>

<style scoped>
.export-overlay {
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

.export-dialog {
  width: 440px;
  max-width: calc(100vw - 48px);
  display: flex;
  flex-direction: column;
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  animation: dialog-scale-in var(--transition-fast) cubic-bezier(0.16, 1, 0.3, 1);
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

.export-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 20px 8px 20px;
}

.export-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: -0.3px;
}

.export-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.export-close:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.export-close:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.export-subtitle {
  margin: 0 20px 12px 20px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text-secondary);
  letter-spacing: -0.2px;
}

.export-field {
  margin: 0 20px 12px 20px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.export-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.export-input {
  height: 34px;
  padding: 0 12px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  font-size: 13px;
  outline: none;
  transition: all var(--transition-fast);
}

.export-input:focus {
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}

.export-input::placeholder {
  color: var(--color-text-tertiary);
}

.export-error {
  margin: 0 20px 12px 20px;
  padding: 10px 12px;
  border: 1px solid var(--color-danger, #ef4444);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--color-danger, #ef4444) 10%, transparent);
  color: var(--color-danger, #ef4444);
  font-size: 13px;
  line-height: 1.5;
}

.export-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 16px 20px;
  border-top: 1px solid var(--color-border-secondary);
  background: var(--color-bg-secondary);
}

.export-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 76px;
  height: 32px;
  padding: 0 16px;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  letter-spacing: -0.2px;
  cursor: pointer;
  transition: all var(--transition-fast);
  outline: none;
}

.export-btn:focus-visible {
  box-shadow: var(--focus-ring);
}

.export-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.export-btn.secondary {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-secondary);
}

.export-btn.secondary:hover {
  background: var(--color-border-primary);
}

.export-btn.primary {
  background: var(--color-primary);
  color: white;
  font-weight: 600;
}

.export-btn.primary:hover {
  background: var(--color-primary-hover);
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
