<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  Activity,
  ArrowDown,
  ArrowUp,
  Cpu,
  MemoryStick,
  HardDrive,
  Network,
} from 'lucide-vue-next';

/**
 * ServerStatusView.vue
 * A floating status bar at the top center of the terminal view.
 * Displays connection info and performance metrics.
 */
interface Props {
  sessionId?: string;
  status?: ServerStatus;
}

const props = defineProps<Props>();
const emit = defineEmits(['toggle-dashboard']);

interface ServerStatus {
  cpuUsage: number;
  memUsage: number;
  memTotal: number;
  memUsed: number;
  diskUsage: number;
  netUp: number;
  netDown: number;
  latency: number;
}

const localStatus = ref<ServerStatus>({
  cpuUsage: 0,
  memUsage: 0,
  memTotal: 0,
  memUsed: 0,
  diskUsage: 0,
  netUp: 0,
  netDown: 0,
  latency: 0,
});

const currentStatus = computed(() => props.status || localStatus.value);

let unlisten: UnlistenFn | null = null;

const formatSpeed = (bytesPerSec: number) => {
  if (!bytesPerSec || bytesPerSec < 0.1) return '0 B/s';
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1024 * 1024)
    return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
};

const formatSize = (bytes: number) => {
  if (!bytes) return '0 MB';
  const mb = bytes / (1024 * 1024);
  if (mb < 1024) return `${mb.toFixed(0)} MB`;
  return `${(mb / 1024).toFixed(1)} GB`;
};

const getResourceColor = (percentage: number) => {
  if (percentage >= 90) return '#ef4444'; // Red
  if (percentage >= 70) return '#f97316'; // Orange
  return '#facc15'; // Default Yellow/Primary
};

const setupListener = async (sid: string) => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }

  if (!sid) return;

  unlisten = await listen<ServerStatus>(`ssh-status-${sid}`, event => {
    localStatus.value = event.payload;
  });
};

onMounted(() => {
  if (props.sessionId && !props.status) {
    setupListener(props.sessionId);
  }
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

watch(
  () => props.sessionId,
  newId => {
    if (newId && !props.status) {
      setupListener(newId);
    } else {
      localStatus.value = {
        cpuUsage: 0,
        memUsage: 0,
        memTotal: 0,
        memUsed: 0,
        diskUsage: 0,
        netUp: 0,
        netDown: 0,
        latency: 0,
      };
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    }
  }
);
</script>

<template>
  <div
    v-if="sessionId"
    class="server-status-view"
    title="Click to toggle detailed dashboard"
    @click="emit('toggle-dashboard')"
  >
    <div class="status-item metrics">
      <Network :size="12" class="icon small" />
      <div class="metric">
        <Activity :size="12" class="icon-metric" />
        <span class="value">{{ currentStatus.latency }}ms</span>
      </div>
      <div class="metric">
        <ArrowDown :size="12" class="icon-metric" />
        <span class="value">{{ formatSpeed(currentStatus.netDown) }}</span>
      </div>
      <div class="metric">
        <ArrowUp :size="12" class="icon-metric" />
        <span class="value">{{ formatSpeed(currentStatus.netUp) }}</span>
      </div>
    </div>

    <div class="divider"></div>

    <div class="status-item resources">
      <div
        class="resource-pill"
        :title="`CPU: ${currentStatus.cpuUsage.toFixed(1)}%`"
      >
        <Cpu :size="12" class="icon small" />
        <span class="label">CPU</span>
        <div class="progress-bg">
          <div
            class="progress-bar"
            :style="{
              width: `${currentStatus.cpuUsage}%`,
              backgroundColor: getResourceColor(currentStatus.cpuUsage),
            }"
          ></div>
        </div>
        <span class="percent">{{ Math.round(currentStatus.cpuUsage) }}%</span>
      </div>
      <div
        class="resource-pill"
        :title="`MEM: ${formatSize(currentStatus.memUsed)} / ${formatSize(currentStatus.memTotal)}`"
      >
        <MemoryStick :size="12" class="icon small" />
        <span class="label">MEM</span>
        <div class="progress-bg">
          <div
            class="progress-bar"
            :style="{
              width: `${currentStatus.memUsage}%`,
              backgroundColor: getResourceColor(currentStatus.memUsage),
            }"
          ></div>
        </div>
        <span class="percent">{{ formatSize(currentStatus.memUsed) }}</span>
      </div>
      <div class="resource-pill" :title="`DISK: ${currentStatus.diskUsage}%`">
        <HardDrive :size="12" class="icon small" />
        <span class="label">DISK</span>
        <div class="progress-bg">
          <div
            class="progress-bar"
            :style="{
              width: `${currentStatus.diskUsage}%`,
              backgroundColor: getResourceColor(currentStatus.diskUsage),
            }"
          ></div>
        </div>
        <span class="percent">{{ Math.round(currentStatus.diskUsage) }}%</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.server-status-view {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  z-index: 90;
  display: flex;
  align-items: center;
  gap: 12px;
  background-color: rgba(30, 30, 30, 0.85);
  backdrop-filter: blur(8px);
  padding: 4px 16px;
  border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-top: none;
  box-shadow: var(--shadow-md);
  color: #d4d4d4;
  font-family: inherit;
  font-size: 11px;
  transition: all var(--transition-base);
  user-select: none;
  pointer-events: auto;
  cursor: pointer;
  animation: slide-down 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

@keyframes slide-down {
  from {
    top: -40px;
    opacity: 0;
  }
  to {
    top: 0;
    opacity: 1;
  }
}

.server-status-view:hover {
  background-color: rgba(40, 40, 40, 0.95);
  border-color: rgba(255, 255, 255, 0.2);
  transform: translateX(-50%) translateY(2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.status-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.icon {
  color: #facc15;
}

.icon-metric {
  color: #888;
  margin-right: 4px;
}

.small {
  color: #888;
}

.divider {
  width: 1px;
  height: 14px;
  background-color: rgba(255, 255, 255, 0.15);
}

.metrics {
  gap: 12px;
}

.metric {
  display: flex;
  align-items: center;
}

.metric .value {
  color: #bbb;
}

.resources {
  gap: 8px;
}

.resource-pill {
  display: flex;
  align-items: center;
  gap: 6px;
}

.resource-pill .label {
  color: #666;
  font-weight: 600;
  font-size: 9px;
}

.percent {
  font-size: 9px;
  opacity: 0.7;
  min-width: 35px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.progress-bg {
  width: 30px;
  height: 4px;
  background-color: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background-color: #facc15;
  border-radius: 2px;
  transition: width 0.3s ease;
}
</style>
