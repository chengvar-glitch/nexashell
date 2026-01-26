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
  if (!bytesPerSec || bytesPerSec < 0.1) return '0B/s';
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)}B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)}K/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)}M/s`;
};

const formatSize = (bytes: number) => {
  if (!bytes) return '0M';
  const mb = bytes / (1024 * 1024);
  if (mb < 1024) return `${mb.toFixed(0)}M`;
  return `${(mb / 1024).toFixed(1)}G`;
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
      <div class="metric" title="Latency">
        <Activity :size="10" class="icon-metric" />
        <span class="value width-30">{{ currentStatus.latency }}ms</span>
      </div>
      <div class="metric" title="Download Speed">
        <ArrowDown :size="10" class="icon-metric" />
        <span class="value width-45">{{
          formatSpeed(currentStatus.netDown)
        }}</span>
      </div>
      <div class="metric" title="Upload Speed">
        <ArrowUp :size="10" class="icon-metric" />
        <span class="value width-45">{{
          formatSpeed(currentStatus.netUp)
        }}</span>
      </div>
    </div>

    <div class="divider"></div>

    <div class="status-item resources">
      <div
        class="metric"
        :title="`CPU Usage: ${currentStatus.cpuUsage.toFixed(1)}%`"
      >
        <Cpu :size="10" class="icon-metric" />
        <span
          class="value width-30"
          :style="{ color: getResourceColor(currentStatus.cpuUsage) }"
        >
          {{ Math.round(currentStatus.cpuUsage) }}%
        </span>
      </div>
      <div
        class="metric"
        :title="`Memory Usage: ${formatSize(currentStatus.memUsed)} / ${formatSize(currentStatus.memTotal)}`"
      >
        <MemoryStick :size="10" class="icon-metric" />
        <span
          class="value width-35"
          :style="{ color: getResourceColor(currentStatus.memUsage) }"
        >
          {{ Math.round(currentStatus.memUsage) }}%
        </span>
      </div>
      <div class="metric" :title="`Disk Usage: ${currentStatus.diskUsage}%`">
        <HardDrive :size="10" class="icon-metric" />
        <span
          class="value width-30"
          :style="{ color: getResourceColor(currentStatus.diskUsage) }"
        >
          {{ Math.round(currentStatus.diskUsage) }}%
        </span>
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
  gap: 6px;
  background-color: rgba(20, 20, 20, 0.65);
  backdrop-filter: blur(12px);
  padding: 2px 8px;
  border-radius: 0 0 var(--radius-md) var(--radius-md);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-top: none;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  color: #888;
  font-family: inherit;
  font-size: 10px;
  transition: all 0.2s ease;
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
  background-color: rgba(30, 30, 30, 0.95);
  border-color: rgba(255, 255, 255, 0.15);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  color: #ccc;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-metric {
  color: #666;
  margin-right: 3px;
  flex-shrink: 0;
}

.divider {
  width: 1px;
  height: 8px;
  background-color: rgba(255, 255, 255, 0.1);
  margin: 0 1px;
}

.metric {
  display: flex;
  align-items: center;
}

.metric .value {
  color: #bbb;
  font-variant-numeric: tabular-nums;
  display: inline-block;
  white-space: nowrap;
}

/* Fixed widths to prevent layout jitter */
.width-30 {
  width: 30px;
}
.width-35 {
  width: 35px;
}
.width-45 {
  width: 50px;
}

.resources {
  gap: 8px;
}
</style>
