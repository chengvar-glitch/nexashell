<template>
  <Teleport to="body">
    <Transition name="tunnel-fade">
      <div
        v-if="visible"
        class="tunnel-overlay"
        role="dialog"
        aria-modal="true"
        aria-label="Port forwarding"
        @click.self="handleClose"
      >
        <div class="tunnel-panel panel" @click.stop>
          <div class="tunnel-header border-bottom">
            <h2 class="tunnel-title">{{ $t('tunnel.title') }}</h2>
            <button
              type="button"
              class="tunnel-close-btn"
              :title="$t('common.cancel')"
              @click="handleClose"
            >
              <X :size="16" />
            </button>
          </div>

          <div class="tunnel-body">
            <!-- Add-rule form -->
            <form class="tunnel-form" novalidate @submit.prevent="handleAddRule">
              <div class="tunnel-form-row">
                <label class="tunnel-field">
                  <span class="tunnel-field-label">{{
                    $t('tunnel.direction')
                  }}</span>
                  <select
                    v-model="draft.direction"
                    class="tunnel-input modal-input"
                  >
                    <option value="local">
                      {{ $t('tunnel.directionLocal') }}
                    </option>
                    <option value="dynamic">
                      {{ $t('tunnel.directionDynamic') }}
                    </option>
                  </select>
                </label>

                <label class="tunnel-field">
                  <span class="tunnel-field-label">{{
                    $t('tunnel.listenHost')
                  }}</span>
                  <input
                    v-model="draft.listenHost"
                    type="text"
                    class="tunnel-input modal-input"
                    placeholder="127.0.0.1"
                  />
                </label>

                <label class="tunnel-field tunnel-field-small">
                  <span class="tunnel-field-label">{{
                    $t('tunnel.listenPort')
                  }}</span>
                  <input
                    v-model.number="draft.listenPort"
                    type="number"
                    min="1"
                    max="65535"
                    class="tunnel-input modal-input"
                  />
                </label>
              </div>

              <div v-if="draft.direction === 'local'" class="tunnel-form-row">
                <label class="tunnel-field">
                  <span class="tunnel-field-label">{{
                    $t('tunnel.targetHost')
                  }}</span>
                  <input
                    v-model="draft.targetHost"
                    type="text"
                    class="tunnel-input modal-input"
                  />
                </label>

                <label class="tunnel-field tunnel-field-small">
                  <span class="tunnel-field-label">{{
                    $t('tunnel.targetPort')
                  }}</span>
                  <input
                    v-model.number="draft.targetPort"
                    type="number"
                    min="1"
                    max="65535"
                    class="tunnel-input modal-input"
                  />
                </label>
              </div>

              <div class="tunnel-form-actions">
                <p v-if="formError" class="tunnel-form-error">
                  <AlertCircle :size="14" />
                  {{ formError }}
                </p>
                <button type="submit" class="btn btn-primary tunnel-add-btn">
                  <Plus :size="14" />
                  {{ $t('tunnel.add') }}
                </button>
              </div>
            </form>

            <!-- Rules list -->
            <div class="tunnel-list">
              <div v-if="rules.length === 0" class="tunnel-empty">
                {{ $t('tunnel.empty') }}
              </div>

              <div
                v-for="rule in rules"
                :key="rule.id"
                class="tunnel-card"
              >
                <div class="tunnel-card-main">
                  <div class="tunnel-summary">
                    <span class="tunnel-direction">{{
                      rule.direction
                    }}</span>
                    <span class="tunnel-listen">
                      {{ rule.listenHost }}:{{ rule.listenPort }}
                    </span>
                    <template v-if="rule.direction === 'local'">
                      <span class="tunnel-arrow">→</span>
                      <span class="tunnel-target">
                        {{ rule.targetHost }}:{{ rule.targetPort }}
                      </span>
                    </template>
                  </div>

                  <div class="tunnel-card-meta">
                    <span
                      class="tunnel-badge"
                      :class="`tunnel-badge-${statusState(rule.id)}`"
                      :title="statusError(rule.id) || undefined"
                    >
                      {{ statusLabel(rule.id) }}
                    </span>
                    <span class="tunnel-accepted">
                      {{ $t('tunnel.accepted') }}:
                      {{ acceptedCount(rule.id) }}
                    </span>
                  </div>

                  <p
                    v-if="statusError(rule.id)"
                    class="tunnel-error-line"
                  >
                    {{ statusError(rule.id) }}
                  </p>
                </div>

                <div class="tunnel-card-actions">
                  <button
                    type="button"
                    class="tunnel-icon-btn"
                    :title="
                      isRunning(rule.id)
                        ? $t('tunnel.stop')
                        : $t('tunnel.start')
                    "
                    @click="handleToggleRule(rule)"
                  >
                    <Square v-if="isRunning(rule.id)" :size="14" />
                    <Play v-else :size="14" />
                  </button>
                  <button
                    type="button"
                    class="tunnel-icon-btn tunnel-icon-btn-danger"
                    :title="$t('tunnel.delete')"
                    @click="handleDeleteRule(rule)"
                  >
                    <Trash2 :size="14" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { onBeforeUnmount, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { AlertCircle, Play, Plus, Square, Trash2, X } from 'lucide-vue-next';
import { tunnelApi } from '@/features/tunnel';
import { createLogger } from '@/core/utils/logger';
import type { TunnelDirection, TunnelRule, TunnelStatus } from '@/features/tunnel';

const logger = createLogger('TUNNEL_PANEL');
const { t } = useI18n();

const props = defineProps<{
  sessionId: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const rules = ref<TunnelRule[]>([]);
const statuses = ref<TunnelStatus[]>([]);
const formError = ref<string>('');
let refreshTimer: ReturnType<typeof setInterval> | null = null;

const draft = reactive<{
  direction: TunnelDirection;
  listenHost: string;
  listenPort: number;
  targetHost: string;
  targetPort: number | null;
}>({
  direction: 'local',
  listenHost: '127.0.0.1',
  listenPort: 8080,
  targetHost: '',
  targetPort: null,
});

const statusByRule = (ruleId: string): TunnelStatus | undefined =>
  statuses.value.find(status => status.ruleId === ruleId);

const isRunning = (ruleId: string): boolean => {
  const status = statusByRule(ruleId);
  return status?.state === 'listening' || status?.state === 'starting';
};

const statusLabel = (ruleId: string): string => {
  const state = statusByRule(ruleId)?.state;
  switch (state) {
    case 'listening':
      return t('tunnel.statusListening');
    case 'failed':
      return t('tunnel.statusFailed');
    case 'starting':
      return t('tunnel.statusStarting');
    case 'stopped':
      return t('tunnel.statusStopped');
    default:
      return t('tunnel.statusStopped');
  }
};

const statusState = (ruleId: string): string => {
  const state = statusByRule(ruleId)?.state;
  if (state === 'listening') return 'listening';
  if (state === 'failed') return 'failed';
  if (state === 'starting') return 'starting';
  return 'stopped';
};

const statusError = (ruleId: string): string | null =>
  statusByRule(ruleId)?.error || null;

const acceptedCount = (ruleId: string): number =>
  statusByRule(ruleId)?.accepted || 0;

const refreshData = async (): Promise<void> => {
  if (!props.sessionId) return;
  try {
    const [nextRules, nextStatuses] = await Promise.all([
      tunnelApi.listTunnelRules(props.sessionId),
      tunnelApi.listTunnelStatus(props.sessionId),
    ]);
    rules.value = nextRules;
    statuses.value = nextStatuses;
  } catch (e) {
    logger.warn('Failed to refresh tunnel rules/status', e);
  }
};

const startRefreshTimer = (): void => {
  if (refreshTimer) {
    clearInterval(refreshTimer);
  }
  refreshTimer = setInterval(() => {
    if (props.sessionId) {
      void tunnelApi
        .listTunnelStatus(props.sessionId)
        .then(list => {
          statuses.value = list;
        })
        .catch(e => logger.warn('Failed to refresh tunnel status', e));
    }
  }, 2000);
};

const stopRefreshTimer = (): void => {
  if (refreshTimer) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
};

const handleAddRule = async (): Promise<void> => {
  if (!props.sessionId) return;
  formError.value = '';

  const listenPort = Number(draft.listenPort);
  if (!Number.isInteger(listenPort) || listenPort < 1 || listenPort > 65535) {
    formError.value = t('tunnel.errorInvalidPort');
    return;
  }

  // Trim the listen host; keep the default 127.0.0.1 semantics when blank
  // (the backend only binds the loopback address by default).
  const listenHost = draft.listenHost.trim() || '127.0.0.1';
  if (!listenHost) {
    formError.value = t('tunnel.errorListenHost');
    return;
  }

  if (draft.direction === 'local') {
    if (!draft.targetHost.trim()) {
      formError.value = t('tunnel.errorTargetRequired');
      return;
    }
    const targetPort = Number(draft.targetPort);
    if (
      draft.targetPort === null ||
      !Number.isInteger(targetPort) ||
      targetPort < 1 ||
      targetPort > 65535
    ) {
      formError.value = t('tunnel.errorInvalidPort');
      return;
    }
  }

  // Conflict check: reject a rule that would bind the same host:port twice.
  const duplicate = rules.value.some(
    r =>
      r.direction === draft.direction &&
      r.listenHost === listenHost &&
      r.listenPort === listenPort
  );
  if (duplicate) {
    formError.value = t('tunnel.errorDuplicate');
    return;
  }

  try {
    const ruleId = await tunnelApi.addTunnelRule({
      sessionId: props.sessionId,
      direction: draft.direction,
      listenHost,
      listenPort: listenPort,
      targetHost:
        draft.direction === 'local' ? draft.targetHost.trim() : '',
      targetPort:
        draft.direction === 'local' ? Number(draft.targetPort) : 0,
      enabled: true,
    });
    await refreshData();
    try {
      await tunnelApi.startTunnelRule(props.sessionId, ruleId);
      statuses.value = await tunnelApi.listTunnelStatus(props.sessionId);
    } catch (e) {
      logger.warn('Failed to start newly added tunnel rule', e);
      formError.value = t('tunnel.errorStartFailed');
    }
    // Reset local form fields
    draft.listenHost = '127.0.0.1';
    draft.listenPort = 8080;
    draft.targetHost = '';
    draft.targetPort = null;
  } catch (error) {
    logger.error('Failed to add tunnel rule', error);
    formError.value = t('tunnel.errorFillRequired');
  }
};

const handleToggleRule = async (rule: TunnelRule): Promise<void> => {
  if (!props.sessionId) return;
  try {
    if (isRunning(rule.id)) {
      await tunnelApi.stopTunnelRule(props.sessionId, rule.id);
    } else {
      await tunnelApi.startTunnelRule(props.sessionId, rule.id);
    }
    statuses.value = await tunnelApi.listTunnelStatus(props.sessionId);
    formError.value = '';
  } catch (error) {
    logger.error('Failed to toggle tunnel rule', error);
    // Surface start/stop failures instead of letting them disappear.
    formError.value = t('tunnel.errorToggleFailed');
  }
};

const handleDeleteRule = async (rule: TunnelRule): Promise<void> => {
  if (!props.sessionId) return;
  try {
    if (isRunning(rule.id)) {
      try {
        await tunnelApi.stopTunnelRule(props.sessionId, rule.id);
      } catch (error) {
        logger.warn('Failed to stop tunnel before delete', error);
      }
    }
    await tunnelApi.deleteTunnelRule(rule.id);
    statuses.value = statuses.value.filter(
      status => status.ruleId !== rule.id
    );
    rules.value = rules.value.filter(item => item.id !== rule.id);
  } catch (error) {
    logger.error('Failed to delete tunnel rule', error);
  }
};

const handleClose = (): void => {
  emit('update:visible', false);
};

watch(
  () => [props.visible, props.sessionId] as const,
  async ([visible]) => {
    if (visible && props.sessionId) {
      await refreshData();
      startRefreshTimer();
    } else {
      stopRefreshTimer();
    }
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  stopRefreshTimer();
});
</script>

<style scoped>
.tunnel-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--color-bg-overlay, rgba(0, 0, 0, 0.5));
}

.tunnel-panel {
  display: flex;
  flex-direction: column;
  width: 520px;
  max-width: calc(100vw - 32px);
  max-height: calc(100vh - 96px);
  background-color: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
}

.tunnel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 18px;
}

.tunnel-title {
  margin: 0;
  font-size: 1.1em;
  font-weight: 600;
  color: var(--color-text-primary);
}

.tunnel-close-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition:
    background 0.15s,
    color 0.15s;
}

.tunnel-close-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.tunnel-body {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.tunnel-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tunnel-form-row {
  display: flex;
  gap: 12px;
  align-items: flex-end;
}

.tunnel-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.tunnel-field-small {
  flex: 0 0 110px;
}

.tunnel-field-label {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.tunnel-input {
  width: 100%;
  padding: 7px 9px;
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-sm);
  background-color: var(--color-bg-secondary);
  color: var(--color-text-primary);
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}

.tunnel-input:focus {
  border-color: var(--color-primary);
}

.tunnel-form-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
}

.tunnel-form-error {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: 12px;
  color: var(--color-danger);
}

.tunnel-add-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background-color: var(--color-primary);
  color: white;
}

.tunnel-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tunnel-empty {
  padding: 24px;
  text-align: center;
  border: 1px dashed var(--color-border-secondary);
  border-radius: var(--radius-md);
  color: var(--color-text-tertiary);
  font-size: 13px;
}

.tunnel-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-secondary);
}

.tunnel-card-main {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.tunnel-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono, monospace);
  font-size: 13px;
  color: var(--color-text-primary);
}

.tunnel-direction {
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  color: var(--color-primary);
  font-size: 11px;
  text-transform: uppercase;
}

.tunnel-arrow {
  color: var(--color-text-tertiary);
}

.tunnel-card-meta {
  display: flex;
  align-items: center;
  gap: 12px;
}

.tunnel-badge {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
}

.tunnel-badge-listening {
  background: color-mix(in srgb, #10b981 18%, transparent);
  color: #10b981;
}

.tunnel-badge-failed {
  background: color-mix(in srgb, #ef4444 18%, transparent);
  color: #ef4444;
}

.tunnel-badge-starting,
.tunnel-badge-stopped {
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
}

.tunnel-accepted {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.tunnel-error-line {
  margin: 0;
  font-size: 12px;
  color: var(--color-danger);
  word-break: break-word;
}

.tunnel-card-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.tunnel-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition:
    background 0.15s,
    color 0.15s,
    border-color 0.15s;
}

.tunnel-icon-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
  border-color: var(--color-border-primary);
}

.tunnel-icon-btn-danger:hover {
  background: color-mix(in srgb, var(--color-danger) 15%, transparent);
  border-color: var(--color-danger);
  color: var(--color-danger);
}

.tunnel-fade-enter-active,
.tunnel-fade-leave-active {
  transition: opacity 0.2s var(--ease-snappy);
}

.tunnel-fade-enter-from,
.tunnel-fade-leave-to {
  opacity: 0;
}
</style>
