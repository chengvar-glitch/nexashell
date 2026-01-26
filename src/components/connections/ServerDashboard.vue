<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  X,
  Cpu,
  MemoryStick,
  Globe,
  HardDrive,
  Activity,
  ArrowDown,
  ArrowUp,
} from 'lucide-vue-next';

interface Props {
  sessionId: string;
  history?: ServerStatus[];
}

const props = defineProps<Props>();
const emit = defineEmits(['close']);

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

const localHistory = ref<ServerStatus[]>([]);
const MAX_HISTORY = 60;
const CHART_WIDTH = 212;
const CHART_HEIGHT = 80;

const historyData = computed(() => props.history || localHistory.value);

const latestStatus = computed(() => {
  if (historyData.value.length === 0) return null;
  return historyData.value[historyData.value.length - 1];
});

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

const getStatusColor = (percentage: number) => {
  if (percentage >= 90) return '#ef4444'; // Red
  if (percentage >= 70) return '#f97316'; // Orange
  return '#10b981'; // Green
};

// SVG Path Generation
const getPoints = (data: number[], max: number) => {
  const step = CHART_WIDTH / (MAX_HISTORY - 1);
  return data.map((val, i) => {
    const x = CHART_WIDTH - (data.length - 1 - i) * step;
    const v = Math.min(max, Math.max(0, val));
    const y = CHART_HEIGHT - (v / (max || 1)) * CHART_HEIGHT;
    return { x, y };
  });
};

const generatePath = (data: number[], max: number = 100) => {
  const points = getPoints(data, max);
  if (points.length === 0) return '';
  return `M ${points.map(p => `${p.x},${p.y}`).join(' L ')}`;
};

const generateAreaPath = (data: number[], max: number = 100) => {
  const points = getPoints(data, max);
  if (points.length === 0) return '';
  const path = `M ${points.map(p => `${p.x},${p.y}`).join(' L ')}`;
  const first = points[0];
  const last = points[points.length - 1];
  return `${path} L ${last.x},CHART_HEIGHT L ${first.x},CHART_HEIGHT Z`.replace(
    /CHART_HEIGHT/g,
    CHART_HEIGHT.toString()
  );
};

const cpuPath = computed(() =>
  generatePath(
    historyData.value.map(h => h.cpuUsage),
    100
  )
);
const cpuAreaPath = computed(() =>
  generateAreaPath(
    historyData.value.map(h => h.cpuUsage),
    100
  )
);

const memPath = computed(() =>
  generatePath(
    historyData.value.map(h => h.memUsage),
    100
  )
);
const memAreaPath = computed(() =>
  generateAreaPath(
    historyData.value.map(h => h.memUsage),
    100
  )
);

const maxNet = computed(() => {
  const values = historyData.value.flatMap(h => [h.netDown, h.netUp]);
  return Math.max(1024 * 100, ...values); // Max scale min 100KB/s
});

const netDownPath = computed(() =>
  generatePath(
    historyData.value.map(h => h.netDown),
    maxNet.value
  )
);
const netDownAreaPath = computed(() =>
  generateAreaPath(
    historyData.value.map(h => h.netDown),
    maxNet.value
  )
);

const netUpPath = computed(() =>
  generatePath(
    historyData.value.map(h => h.netUp),
    maxNet.value
  )
);
const netUpAreaPath = computed(() =>
  generateAreaPath(
    historyData.value.map(h => h.netUp),
    maxNet.value
  )
);

let unlisten: UnlistenFn | null = null;

const setupListener = async (sid: string) => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }

  if (props.history) return;

  unlisten = await listen<ServerStatus>(`ssh-status-${sid}`, event => {
    localHistory.value.push(event.payload);
    if (localHistory.value.length > MAX_HISTORY) {
      localHistory.value.shift();
    }
  });
};

onMounted(async () => {
  await nextTick();
  setupListener(props.sessionId);
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

watch(
  () => props.sessionId,
  newId => {
    localHistory.value = [];
    if (!props.history) {
      setupListener(newId);
    }
  }
);
</script>

<template>
  <div class="server-dashboard">
    <div class="dashboard-header">
      <div class="title-group">
        <Activity :size="14" class="header-icon" />
        <span class="title">System Performance</span>
      </div>
      <button class="close-btn" @click="emit('close')">
        <X :size="16" />
      </button>
    </div>

    <div class="charts-container">
      <!-- CPU Section -->
      <div class="metric-section">
        <div class="metric-header">
          <div class="metric-label">
            <Cpu :size="14" class="metric-icon" />
            <span>Processor (CPU)</span>
          </div>
          <span
            v-if="latestStatus"
            class="metric-value"
            :style="{ color: getStatusColor(latestStatus.cpuUsage) }"
          >
            {{ latestStatus.cpuUsage.toFixed(1) }}%
          </span>
        </div>
        <div class="chart-container-svg">
          <svg
            class="svg-chart"
            :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
            preserveAspectRatio="none"
          >
            <!-- Grid Lines -->
            <line x1="0" y1="0" :x2="CHART_WIDTH" y2="0" class="grid-line" />
            <line x1="0" y1="20" :x2="CHART_WIDTH" y2="20" class="grid-line" />
            <line x1="0" y1="40" :x2="CHART_WIDTH" y2="40" class="grid-line" />
            <line x1="0" y1="60" :x2="CHART_WIDTH" y2="60" class="grid-line" />
            <line
              x1="0"
              :y1="CHART_HEIGHT"
              :x2="CHART_WIDTH"
              :y2="CHART_HEIGHT"
              class="grid-line"
            />

            <!-- Data Area -->
            <path :d="cpuAreaPath" fill="url(#cpuGradient)" />
            <!-- Data Line -->
            <path
              :d="cpuPath"
              stroke="#3b82f6"
              stroke-width="1.5"
              fill="none"
            />

            <defs>
              <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.3" />
                <stop offset="100%" stop-color="#3b82f6" stop-opacity="0" />
              </linearGradient>
            </defs>
          </svg>
        </div>
      </div>

      <!-- Memory Section -->
      <div class="metric-section">
        <div class="metric-header">
          <div class="metric-label">
            <MemoryStick :size="14" class="metric-icon" />
            <span>Memory (RAM)</span>
          </div>
          <span
            v-if="latestStatus"
            class="metric-value"
            :style="{ color: getStatusColor(latestStatus.memUsage) }"
          >
            {{ latestStatus.memUsage.toFixed(1) }}%
          </span>
        </div>
        <div v-if="latestStatus" class="mem-details">
          {{ formatSize(latestStatus.memUsed) }} /
          {{ formatSize(latestStatus.memTotal) }}
        </div>
        <div class="chart-container-svg">
          <svg
            class="svg-chart"
            :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
            preserveAspectRatio="none"
          >
            <line x1="0" y1="0" :x2="CHART_WIDTH" y2="0" class="grid-line" />
            <line x1="0" y1="20" :x2="CHART_WIDTH" y2="20" class="grid-line" />
            <line x1="0" y1="40" :x2="CHART_WIDTH" y2="40" class="grid-line" />
            <line x1="0" y1="60" :x2="CHART_WIDTH" y2="60" class="grid-line" />
            <line
              x1="0"
              :y1="CHART_HEIGHT"
              :x2="CHART_WIDTH"
              :y2="CHART_HEIGHT"
              class="grid-line"
            />

            <path :d="memAreaPath" fill="url(#memGradient)" />
            <path
              :d="memPath"
              stroke="#10b981"
              stroke-width="1.5"
              fill="none"
            />

            <defs>
              <linearGradient id="memGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#10b981" stop-opacity="0.3" />
                <stop offset="100%" stop-color="#10b981" stop-opacity="0" />
              </linearGradient>
            </defs>
          </svg>
        </div>
      </div>

      <!-- Network Section -->
      <div class="metric-section">
        <div class="metric-header">
          <div class="metric-label">
            <Globe :size="14" class="metric-icon" />
            <span>Network Traffic</span>
          </div>
        </div>
        <div v-if="latestStatus" class="net-details">
          <div class="net-item down">
            <ArrowDown :size="12" />
            <span>{{ formatSpeed(latestStatus.netDown) }}</span>
          </div>
          <div class="net-item up">
            <ArrowUp :size="12" />
            <span>{{ formatSpeed(latestStatus.netUp) }}</span>
          </div>
        </div>
        <div class="chart-container-svg">
          <svg
            class="svg-chart"
            :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
            preserveAspectRatio="none"
          >
            <line x1="0" y1="0" :x2="CHART_WIDTH" y2="0" class="grid-line" />
            <line x1="0" y1="20" :x2="CHART_WIDTH" y2="20" class="grid-line" />
            <line x1="0" y1="40" :x2="CHART_WIDTH" y2="40" class="grid-line" />
            <line x1="0" y1="60" :x2="CHART_WIDTH" y2="60" class="grid-line" />
            <line
              x1="0"
              :y1="CHART_HEIGHT"
              :x2="CHART_WIDTH"
              :y2="CHART_HEIGHT"
              class="grid-line"
            />

            <!-- Down Traffic -->
            <path :d="netDownAreaPath" fill="url(#netDownGradient)" />
            <path
              :d="netDownPath"
              stroke="#3b82f6"
              stroke-width="1.5"
              fill="none"
            />

            <!-- Up Traffic -->
            <path :d="netUpAreaPath" fill="url(#netUpGradient)" />
            <path
              :d="netUpPath"
              stroke="#f59e0b"
              stroke-width="1.5"
              fill="none"
            />

            <defs>
              <linearGradient id="netDownGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.15" />
                <stop offset="100%" stop-color="#3b82f6" stop-opacity="0" />
              </linearGradient>
              <linearGradient id="netUpGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.15" />
                <stop offset="100%" stop-color="#f59e0b" stop-opacity="0" />
              </linearGradient>
            </defs>
          </svg>
        </div>
      </div>
    </div>

    <div class="dashboard-footer">
      <div class="footer-item">
        <Activity :size="12" class="footer-icon" />
        <span class="label">Latency:</span>
        <span class="value">{{ latestStatus?.latency || 0 }}ms</span>
      </div>
      <div class="footer-item">
        <HardDrive :size="12" class="footer-icon" />
        <span class="label">Disk:</span>
        <span class="value">{{ latestStatus?.diskUsage || 0 }}%</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.server-dashboard {
  position: absolute;
  top: 40px;
  right: 12px;
  width: 240px;
  background-color: rgba(15, 15, 15, 0.45);
  backdrop-filter: blur(8px);
  border-radius: var(--radius-md);
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  z-index: 100;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: slide-in 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

@keyframes slide-in {
  from {
    opacity: 0;
    transform: translateX(20px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  background-color: rgba(255, 255, 255, 0.03);
}

.title-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-icon {
  color: #3b82f6;
}

.dashboard-header .title {
  font-size: 11px;
  font-weight: 600;
  color: #bbb;
  text-transform: uppercase;
  letter-spacing: 0.8px;
}

.close-btn {
  background: none;
  border: none;
  color: #555;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.close-btn:hover {
  background-color: rgba(255, 255, 255, 0.05);
  color: #ef4444;
}

.charts-container {
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.metric-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.metric-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.metric-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #999;
  font-weight: 500;
}

.metric-icon {
  color: #666;
}

.metric-value {
  font-size: 12px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.mem-details,
.net-details {
  font-size: 10px;
  color: #666;
  margin-top: -2px;
}

.net-details {
  display: flex;
  gap: 12px;
}

.net-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.net-item.down {
  color: #3b82f6;
}
.net-item.up {
  color: #f59e0b;
}

.chart-container-svg {
  width: 100%;
  height: 80px;
  margin-top: 4px;
}

.svg-chart {
  width: 100%;
  height: 100%;
  overflow: visible;
}

.grid-line {
  stroke: rgba(255, 255, 255, 0.05);
  stroke-width: 1;
  stroke-dasharray: 2, 2;
}

.dashboard-footer {
  display: flex;
  justify-content: space-between;
  padding: 10px 16px;
  background-color: rgba(0, 0, 0, 0.2);
  border-top: 1px solid rgba(255, 255, 255, 0.05);
}

.footer-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.footer-icon {
  color: #555;
}

.footer-item .label {
  font-size: 10px;
  color: #555;
  font-weight: 600;
}

.footer-item .value {
  font-size: 11px;
  color: #888;
  font-variant-numeric: tabular-nums;
}
</style>
