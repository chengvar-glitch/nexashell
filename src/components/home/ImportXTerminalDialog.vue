<template>
  <div v-if="visible" class="import-overlay" @click.self="handleCancel">
    <div class="import-dialog" role="dialog" aria-modal="true" :aria-label="t('import.title')">
      <div class="import-header">
        <h3 class="import-title">{{ t('import.title') }}</h3>
        <button
          class="import-close"
          :disabled="busy"
          :title="t('common.cancel')"
          @click="handleCancel"
        >
          <X :size="16" />
        </button>
      </div>

      <div class="import-tabs" role="tablist">
        <button
          class="import-tab"
          role="tab"
          :class="{ active: mode === 'text' }"
          :aria-selected="mode === 'text'"
          @click="mode = 'text'"
        >
          {{ t('import.tabText') }}
        </button>
        <button
          class="import-tab"
          role="tab"
          :class="{ active: mode === 'backup' }"
          :aria-selected="mode === 'backup'"
          @click="mode = 'backup'"
        >
          {{ t('import.tabBackup') }}
        </button>
      </div>

      <template v-if="mode === 'text'">
        <p class="import-subtitle">{{ t('import.subtitle') }}</p>

        <textarea
          ref="textareaRef"
          v-model="text"
          class="import-textarea"
          :placeholder="t('import.placeholder')"
          spellcheck="false"
          @keydown.esc="handleCancel"
        ></textarea>
      </template>

      <template v-else>
        <p class="import-subtitle">{{ t('import.backupHint') }}</p>

        <div class="import-backup-file">
          <button
            class="import-btn secondary"
            :disabled="busy"
            @click="handleChooseFile"
          >
            <FileJson :size="14" />
            {{ t('import.chooseFile') }}
          </button>
          <span v-if="backupFileName" class="import-file-name">
            {{ t('import.fileLoaded', { name: backupFileName }) }}
          </span>
        </div>

        <div class="import-password-field">
          <label class="import-label" for="import-backup-password">
            {{ t('import.backupPassword') }}
          </label>
          <input
            id="import-backup-password"
            ref="backupPasswordRef"
            v-model="backupPassword"
            type="password"
            class="import-password-input"
            autocomplete="off"
            :placeholder="t('import.backupPasswordPlaceholder')"
            :disabled="busy"
            @keydown.esc="handleCancel"
            @keydown.enter="handleImport"
          />
        </div>
      </template>

      <div v-if="errorMessage" class="import-error" role="alert">
        {{ errorMessage }}
      </div>

      <div v-if="result" class="import-result">
        <p v-if="result.imported > 0" class="import-result-line success">
          {{ t('import.success', { imported: result.imported }) }}
        </p>
        <p v-if="result.skipped > 0" class="import-result-line skipped">
          {{ t('import.skipped', { skipped: result.skipped }) }}
        </p>
        <p v-if="result.failed.length > 0" class="import-result-line failed">
          {{ t('import.failed', { failed: result.failed.length }) }}
        </p>
        <p
          v-if="result.imported === 0 && result.skipped === 0 && result.failed.length === 0"
          class="import-result-line empty"
        >
          {{ t('import.empty') }}
        </p>
        <ul v-if="result.failed.length > 0" class="import-error-list">
          <li v-for="(err, i) in result.failed" :key="i">{{ err }}</li>
        </ul>
      </div>

      <div class="import-footer">
        <button class="import-btn secondary" :disabled="busy" @click="handleCancel">
          {{ t('common.cancel') }}
        </button>
        <button
          v-if="mode === 'text'"
          class="import-btn primary"
          :disabled="busy || !text.trim()"
          @click="handleImport"
        >
          <Loader2 v-if="busy" :size="14" class="spin" />
          {{ busy ? t('import.importing') : t('import.confirm') }}
        </button>
        <button
          v-else
          class="import-btn primary"
          :disabled="busy || !backupJson || !backupPassword"
          @click="handleImport"
        >
          <Loader2 v-if="busy" :size="14" class="spin" />
          {{ busy ? t('import.importing') : t('import.confirm') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { FileJson, Loader2, X } from 'lucide-vue-next';
import { sessionApi } from '@/features/session';
import type { ImportResult } from '@/features/session/types';

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  imported: [];
}>();

const { t } = useI18n();

// 'text': paste XTerminal-format text; 'backup': restore a NexaShell
// encrypted export (.json) produced by the batch-export feature.
const mode = ref<'text' | 'backup'>('text');

const textareaRef = ref<HTMLTextAreaElement | null>(null);
const backupPasswordRef = ref<HTMLInputElement | null>(null);
const text = ref('');
const backupJson = ref('');
const backupFileName = ref('');
const backupPassword = ref('');
const busy = ref(false);
const result = ref<ImportResult | null>(null);
const errorMessage = ref('');

watch(
  () => props.visible,
  async value => {
    if (!value) return;
    mode.value = 'text';
    text.value = '';
    backupJson.value = '';
    backupFileName.value = '';
    backupPassword.value = '';
    result.value = null;
    errorMessage.value = '';
    await nextTick();
    textareaRef.value?.focus();
  }
);

watch(mode, async value => {
  result.value = null;
  errorMessage.value = '';
  if (value === 'backup') {
    await nextTick();
    backupPasswordRef.value?.focus();
  }
});

const handleChooseFile = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const path = await open({
      multiple: false,
      filters: [{ name: 'NexaShell Backup', extensions: ['json'] }],
    });
    if (!path || typeof path !== 'string') return;

    const contents = await invoke<string>('read_text_file', { path });
    // Cheap up-front validity check so a wrong file fails here with a clear
    // message instead of a backend parse error after the password is typed.
    const parsed = JSON.parse(contents);
    if (!parsed || !Array.isArray(parsed.sessions)) {
      throw new Error('not an export payload');
    }
    backupJson.value = contents;
    backupFileName.value = path.split(/[\\/]/).pop() || path;
    errorMessage.value = '';
  } catch (error) {
    backupJson.value = '';
    backupFileName.value = '';
    errorMessage.value = t('import.invalidBackup');
    void error;
  }
};

const handleImport = async () => {
  if (busy.value) return;
  if (mode.value === 'text' && !text.value.trim()) return;
  if (mode.value === 'backup' && (!backupJson.value || !backupPassword.value)) {
    return;
  }

  busy.value = true;
  errorMessage.value = '';
  result.value = null;
  try {
    if (mode.value === 'text') {
      result.value = await sessionApi.importXTerminal(text.value);
    } else {
      const imported = await sessionApi.importBackup(
        backupJson.value,
        backupPassword.value
      );
      result.value = { imported, skipped: 0, failed: [] };
    }
    if (result.value.imported > 0) {
      emit('imported');
    }
  } catch (error) {
    errorMessage.value = t('import.error', {
      message: error instanceof Error ? error.message : String(error),
    });
  } finally {
    busy.value = false;
  }
};

const handleCancel = () => {
  if (busy.value) return;
  text.value = '';
  backupJson.value = '';
  backupFileName.value = '';
  backupPassword.value = '';
  result.value = null;
  errorMessage.value = '';
  emit('update:visible', false);
};
</script>

<style scoped>
.import-overlay {
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

.import-dialog {
  width: 560px;
  max-width: calc(100vw - 48px);
  max-height: calc(100vh - 48px);
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

.import-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 20px 8px 20px;
}

.import-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: -0.3px;
}

.import-close {
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

.import-close:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.import-close:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.import-tabs {
  display: flex;
  gap: 4px;
  padding: 0 20px 12px 20px;
  border-bottom: 1px solid var(--color-border-secondary);
}

.import-tab {
  padding: 6px 12px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.import-tab:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.import-tab.active {
  background: var(--color-interactive-selected);
  color: var(--color-primary);
  font-weight: 600;
}

.import-subtitle {
  margin: 12px 20px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text-secondary);
  letter-spacing: -0.2px;
}

.import-textarea {
  flex: 1;
  min-height: 200px;
  margin: 0 20px;
  padding: 12px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  font-family: var(--font-mono, ui-monospace, 'SF Mono', Menlo, Consolas, monospace);
  font-size: 13px;
  line-height: 1.6;
  resize: vertical;
  outline: none;
  transition: all var(--transition-fast);
}

.import-textarea:focus {
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}

.import-textarea::placeholder {
  color: var(--color-text-tertiary);
  font-family: inherit;
}

.import-backup-file {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 20px 12px 20px;
  min-height: 32px;
}

.import-file-name {
  font-size: 12px;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.import-password-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0 20px;
}

.import-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.import-password-input {
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

.import-password-input:focus {
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}

.import-password-input::placeholder {
  color: var(--color-text-tertiary);
}

.import-error {
  margin: 12px 20px 0 20px;
  padding: 10px 12px;
  border: 1px solid var(--color-danger, #ef4444);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--color-danger, #ef4444) 10%, transparent);
  color: var(--color-danger, #ef4444);
  font-size: 13px;
  line-height: 1.5;
  word-break: break-all;
}

.import-result {
  margin: 12px 20px 0 20px;
  padding: 10px 12px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background: var(--color-bg-secondary);
}

.import-result-line {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  letter-spacing: -0.2px;
}

.import-result-line.success {
  color: var(--color-success, #10b981);
}

.import-result-line.skipped {
  color: var(--color-text-secondary);
}

.import-result-line.failed,
.import-result-line.empty {
  color: var(--color-danger, #ef4444);
}

.import-error-list {
  margin: 6px 0 0 0;
  padding-left: 18px;
  max-height: 120px;
  overflow-y: auto;
}

.import-error-list li {
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text-secondary);
  word-break: break-all;
}

.import-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 16px 20px;
  margin-top: 16px;
  border-top: 1px solid var(--color-border-secondary);
  background: var(--color-bg-secondary);
}

.import-btn {
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

.import-btn:hover {
  transform: translateY(-1px);
}

.import-btn:active {
  transform: translateY(0);
}

.import-btn:focus-visible {
  box-shadow: var(--focus-ring);
}

.import-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.import-btn.secondary {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-secondary);
}

.import-btn.secondary:hover {
  background: var(--color-border-primary);
}

.import-btn.primary {
  background: var(--color-primary);
  color: white;
  font-weight: 600;
}

.import-btn.primary:hover {
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
