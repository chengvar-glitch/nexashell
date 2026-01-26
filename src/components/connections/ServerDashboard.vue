<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

const { t } = useI18n();

import {
  Cpu,
  MemoryStick,
  Globe,
  HardDrive,
  Activity,
  ArrowDown,
  ArrowUp,
  Clock,
  Trash2,
  ChevronRight,
  ChevronDown,
  ChevronLeft,
  CheckCircle2,
  AlertCircle,
  FileUp,
  LayoutDashboard,
} from 'lucide-vue-next';

interface UploadTask {
  id: string;
  fileName: string;
  remotePath?: string;
  status: 'pending' | 'uploading' | 'success' | 'error';
  progress: number;
  message: string;
  timestamp: number;
  error?: string;
  fileSize?: number;
  uploadedBytes?: number;
  startTime?: number;
  speed?: number;
  eta?: number;
}

interface Props {
  show: boolean;
  activeTab: 'system' | 'uploads' | null;
  sessionId: string;
  history?: ServerStatus[];
  uploadTasks?: UploadTask[];
}

const props = defineProps<Props>();
const emit = defineEmits([
  'close',
  'clear-tasks',
  'toggle',
  'update:active-tab',
]);

interface ServerStatus {
  cpuUsage: number;
  memUsage: number;
  memTotal: number;
  memUsed: number;
  memAvail: number;
  swapUsage: number;
  swapTotal: number;
  swapUsed: number;
  diskUsage: number;
  diskTotal: number;
  diskUsed: number;
  diskAvail: number;
  netUp: number;
  netDown: number;
  latency: number;
  loadAvg: [number, number, number];
  uptime: string;
}

const localHistory = ref<ServerStatus[]>([]);
const MAX_HISTORY = 60;
const CHART_WIDTH = 212;
const CHART_HEIGHT = 80;

// Accordion state for system metrics
const expandedMetrics = ref({
  cpu: true,
  memory: true,
  network: true,
  disk: true,
});

const historyData = computed(() => props.history || localHistory.value);
const uploadTasksData = computed(() => props.uploadTasks || []);

const hasActiveUploads = computed(() =>
  uploadTasksData.value.some(
    t => t.status === 'uploading' || t.status === 'pending'
  )
);

const toggleTab = (tab: 'system' | 'uploads') => {
  if (props.activeTab === tab) {
    emit('update:active-tab', null);
  } else {
    emit('update:active-tab', tab);
  }
};

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
  if (!bytes) return '0 MiB';
  const mb = bytes / (1024 * 1024);
  if (mb < 1024) return `${mb.toFixed(1)} MiB`;
  return `${(mb / 1024).toFixed(1)} GiB`;
};

const getStatusColor = (percentage: number) => {
  if (percentage >= 90) return '#ef4444'; // Red
  if (percentage >= 70) return '#f97316'; // Orange
  return '#10b981'; // Green
};

const getLatencyColor = (ms: number) => {
  if (ms >= 150) return '#ef4444';
  if (ms >= 60) return '#f97316';
  return '#10b981';
};

const getTaskStatusColor = (status: UploadTask['status']) => {
  switch (status) {
    case 'success':
      return '#10b981'; // Green
    case 'error':
      return '#ef4444'; // Red
    case 'uploading':
      return '#3b82f6'; // Blue
    case 'pending':
      return '#f97316'; // Orange
    default:
      return '#6b7280'; // Gray
  }
};

const getTaskStatusIcon = (status: UploadTask['status']) => {
  switch (status) {
    case 'success':
      return CheckCircle2;
    case 'error':
      return AlertCircle;
    case 'uploading':
      return Activity;
    case 'pending':
      return Clock;
    default:
      return Clock;
  }
};

const formatTaskTime = (timestamp: number) => {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (seconds < 60) return `${seconds}s ${t('dashboard.ago')}`;
  if (minutes < 60) return `${minutes}m ${t('dashboard.ago')}`;
  if (hours < 24) return `${hours}h ${t('dashboard.ago')}`;
  return date.toLocaleDateString();
};

const formatFileSize = (bytes: number): string => {
  if (!bytes) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
};

const formatETASeconds = (seconds: number): string => {
  if (!seconds || seconds <= 0) return '--:--';
  const minutes = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${minutes}m ${secs}s`;
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
  <div class="server-dashboard" :class="{ 'is-hidden': !show }">
    <button
      class="toggle-handle"
      :class="{ 'has-active': hasActiveUploads && !show }"
      :title="show ? t('dashboard.hideSidebar') : t('dashboard.showSidebar')"
      @click="emit('toggle')"
    >
      <ChevronRight v-if="show" class="handle-icon" />
      <ChevronLeft v-else class="handle-icon" />
    </button>

    <div class="dashboard-header">
      <div class="title-group">
        <div class="header-icon-wrapper">
          <LayoutDashboard :size="16" class="header-icon" />
          <div class="icon-glow"></div>
        </div>
        <div class="title-meta">
          <span class="title">{{ t('dashboard.statusCenter') }}</span>
          <span v-if="latestStatus" class="uptime-text">{{
            latestStatus.uptime
          }}</span>
        </div>
      </div>
    </div>

    <div class="accordion-container">
      <!-- System Section -->
      <div
        class="accordion-item"
        :class="{ 'is-active': activeTab === 'system' }"
      >
        <button class="accordion-header" @click="toggleTab('system')">
          <ChevronDown :size="14" class="chevron" />
          <Activity :size="14" class="section-icon" />
          <span class="section-title">{{ t('dashboard.performance') }}</span>
          <span
            v-if="latestStatus && activeTab !== 'system'"
            class="header-badge"
          >
            {{ Math.round(latestStatus.cpuUsage) }}%
          </span>
        </button>

        <div v-if="activeTab === 'system'" class="accordion-content">
          <!-- CPU Section -->
          <div class="metric-section">
            <div class="metric-header">
              <button
                class="metric-toggle"
                @click="expandedMetrics.cpu = !expandedMetrics.cpu"
              >
                <ChevronRight
                  v-if="!expandedMetrics.cpu"
                  :size="14"
                  class="chevron"
                />
                <ChevronDown v-else :size="14" class="chevron" />
              </button>
              <div class="metric-label">
                <Cpu :size="14" class="metric-icon" />
                <span>{{ t('dashboard.processor') }}</span>
              </div>
              <span
                v-if="latestStatus"
                class="metric-value"
                :style="{ color: getStatusColor(latestStatus.cpuUsage) }"
              >
                {{ latestStatus.cpuUsage.toFixed(1) }}%
              </span>
            </div>
            <div v-if="expandedMetrics.cpu" class="metric-details-rows">
              <div v-if="latestStatus" class="load-avg-row">
                <span class="sub-label">{{ t('dashboard.loadAvg') }}:</span>
                <span class="load-values">
                  {{ latestStatus.loadAvg[0].toFixed(2) }}
                  {{ latestStatus.loadAvg[1].toFixed(2) }}
                  {{ latestStatus.loadAvg[2].toFixed(2) }}
                </span>
              </div>
              <div class="chart-container-svg">
                <svg
                  class="svg-chart"
                  :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
                  preserveAspectRatio="none"
                >
                  <line
                    x1="0"
                    y1="0"
                    :x2="CHART_WIDTH"
                    y2="0"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="20"
                    :x2="CHART_WIDTH"
                    y2="20"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="40"
                    :x2="CHART_WIDTH"
                    y2="40"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="60"
                    :x2="CHART_WIDTH"
                    y2="60"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    :y1="CHART_HEIGHT"
                    :x2="CHART_WIDTH"
                    :y2="CHART_HEIGHT"
                    class="grid-line"
                  />
                  <path :d="cpuAreaPath" fill="url(#cpuGradient)" />
                  <path
                    :d="cpuPath"
                    stroke="#3b82f6"
                    stroke-width="1.5"
                    fill="none"
                  />
                  <defs>
                    <linearGradient
                      id="cpuGradient"
                      x1="0"
                      y1="0"
                      x2="0"
                      y2="1"
                    >
                      <stop
                        offset="0%"
                        stop-color="#3b82f6"
                        stop-opacity="0.3"
                      />
                      <stop
                        offset="100%"
                        stop-color="#3b82f6"
                        stop-opacity="0"
                      />
                    </linearGradient>
                  </defs>
                </svg>
              </div>
            </div>
          </div>

          <!-- Memory Section -->
          <div class="metric-section">
            <div class="metric-header">
              <button
                class="metric-toggle"
                @click="expandedMetrics.memory = !expandedMetrics.memory"
              >
                <ChevronRight
                  v-if="!expandedMetrics.memory"
                  :size="14"
                  class="chevron"
                />
                <ChevronDown v-else :size="14" class="chevron" />
              </button>
              <div class="metric-label">
                <MemoryStick :size="14" class="metric-icon" />
                <span>{{ t('dashboard.memory') }}</span>
              </div>
              <span
                v-if="latestStatus"
                class="metric-value"
                :style="{ color: getStatusColor(latestStatus.memUsage) }"
              >
                {{ latestStatus.memUsage.toFixed(1) }}%
              </span>
            </div>
            <div v-if="expandedMetrics.memory">
              <div v-if="latestStatus" class="mem-details">
                <div class="mem-row">
                  <span class="sub-label"
                    >{{ t('dashboard.actualUsed') }}:</span
                  >
                  <span
                    >{{
                      formatSize(latestStatus.memTotal - latestStatus.memAvail)
                    }}
                    / {{ formatSize(latestStatus.memTotal) }}</span
                  >
                </div>
                <div v-if="latestStatus.swapTotal > 0" class="mem-row swap">
                  <span class="sub-label">{{ t('dashboard.swap') }}:</span>
                  <span
                    >{{ formatSize(latestStatus.swapUsed) }} /
                    {{ formatSize(latestStatus.swapTotal) }}</span
                  >
                </div>
              </div>
              <div class="chart-container-svg">
                <svg
                  class="svg-chart"
                  :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
                  preserveAspectRatio="none"
                >
                  <line
                    x1="0"
                    y1="0"
                    :x2="CHART_WIDTH"
                    y2="0"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="20"
                    :x2="CHART_WIDTH"
                    y2="20"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="40"
                    :x2="CHART_WIDTH"
                    y2="40"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="60"
                    :x2="CHART_WIDTH"
                    y2="60"
                    class="grid-line"
                  />
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
                    <linearGradient
                      id="memGradient"
                      x1="0"
                      y1="0"
                      x2="0"
                      y2="1"
                    >
                      <stop
                        offset="0%"
                        stop-color="#10b981"
                        stop-opacity="0.3"
                      />
                      <stop
                        offset="100%"
                        stop-color="#10b981"
                        stop-opacity="0"
                      />
                    </linearGradient>
                  </defs>
                </svg>
              </div>
            </div>
          </div>

          <!-- Network Section -->
          <div class="metric-section">
            <div class="metric-header">
              <button
                class="metric-toggle"
                @click="expandedMetrics.network = !expandedMetrics.network"
              >
                <ChevronRight
                  v-if="!expandedMetrics.network"
                  :size="14"
                  class="chevron"
                />
                <ChevronDown v-else :size="14" class="chevron" />
              </button>
              <div class="metric-label">
                <Globe :size="14" class="metric-icon" />
                <span>{{ t('dashboard.network') }}</span>
              </div>
            </div>
            <div v-if="expandedMetrics.network">
              <div v-if="latestStatus" class="net-details">
                <div class="net-item down">
                  <ArrowDown :size="12" />
                  <span>{{ formatSpeed(latestStatus.netDown) }}</span>
                </div>
                <div class="net-item up">
                  <ArrowUp :size="12" />
                  <span>{{ formatSpeed(latestStatus.netUp) }}</span>
                </div>
                <div class="net-latency">
                  <Activity
                    :size="12"
                    :style="{ color: getLatencyColor(latestStatus.latency) }"
                  />
                  <span
                    :style="{ color: getLatencyColor(latestStatus.latency) }"
                    >{{ latestStatus.latency }}ms</span
                  >
                </div>
              </div>
              <div class="chart-container-svg">
                <svg
                  class="svg-chart"
                  :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
                  preserveAspectRatio="none"
                >
                  <line
                    x1="0"
                    y1="0"
                    :x2="CHART_WIDTH"
                    y2="0"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="20"
                    :x2="CHART_WIDTH"
                    y2="20"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="40"
                    :x2="CHART_WIDTH"
                    y2="40"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    y1="60"
                    :x2="CHART_WIDTH"
                    y2="60"
                    class="grid-line"
                  />
                  <line
                    x1="0"
                    :y1="CHART_HEIGHT"
                    :x2="CHART_WIDTH"
                    :y2="CHART_HEIGHT"
                    class="grid-line"
                  />
                  <path :d="netDownAreaPath" fill="url(#netDownGradient)" />
                  <path
                    :d="netDownPath"
                    stroke="#3b82f6"
                    stroke-width="1.5"
                    fill="none"
                  />
                  <path :d="netUpAreaPath" fill="url(#netUpGradient)" />
                  <path
                    :d="netUpPath"
                    stroke="#f59e0b"
                    stroke-width="1.5"
                    fill="none"
                  />
                  <defs>
                    <linearGradient
                      id="netDownGradient"
                      x1="0"
                      y1="0"
                      x2="0"
                      y2="1"
                    >
                      <stop
                        offset="0%"
                        stop-color="#3b82f6"
                        stop-opacity="0.15"
                      />
                      <stop
                        offset="100%"
                        stop-color="#3b82f6"
                        stop-opacity="0"
                      />
                    </linearGradient>
                    <linearGradient
                      id="netUpGradient"
                      x1="0"
                      y1="0"
                      x2="0"
                      y2="1"
                    >
                      <stop
                        offset="0%"
                        stop-color="#f59e0b"
                        stop-opacity="0.15"
                      />
                      <stop
                        offset="100%"
                        stop-color="#f59e0b"
                        stop-opacity="0"
                      />
                    </linearGradient>
                  </defs>
                </svg>
              </div>
            </div>
          </div>

          <!-- Disk Section -->
          <div class="metric-section">
            <div class="metric-header">
              <button
                class="metric-toggle"
                @click="expandedMetrics.disk = !expandedMetrics.disk"
              >
                <ChevronRight
                  v-if="!expandedMetrics.disk"
                  :size="14"
                  class="chevron"
                />
                <ChevronDown v-else :size="14" class="chevron" />
              </button>
              <div class="metric-label">
                <HardDrive :size="14" class="metric-icon" />
                <span>{{ t('dashboard.disk') }}</span>
              </div>
              <span
                v-if="latestStatus"
                class="metric-value"
                :style="{ color: getStatusColor(latestStatus.diskUsage) }"
              >
                {{ latestStatus.diskUsage.toFixed(1) }}%
              </span>
            </div>
            <div
              v-if="expandedMetrics.disk && latestStatus"
              class="disk-details-container"
            >
              <div class="pie-chart-box">
                <svg viewBox="0 0 36 36" class="circular-chart">
                  <path
                    class="circle-bg"
                    d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                  />
                  <path
                    class="circle"
                    :stroke-dasharray="`${latestStatus.diskUsage}, 100`"
                    :stroke="getStatusColor(latestStatus.diskUsage)"
                    d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                  />
                  <text x="18" y="20.35" class="percentage">
                    {{ Math.round(latestStatus.diskUsage) }}%
                  </text>
                </svg>
              </div>
              <div class="disk-info-list">
                <div class="footer-item usage-details">
                  <div class="capacity-info">
                    <span class="sub-label">{{ t('dashboard.used') }}:</span>
                    {{ formatSize(latestStatus.diskUsed) }}
                  </div>
                  <div class="capacity-info">
                    <span class="sub-label">{{ t('dashboard.total') }}:</span>
                    {{ formatSize(latestStatus.diskTotal) }}
                  </div>
                  <div class="available-text">
                    {{ formatSize(latestStatus.diskAvail) }}
                    {{ t('dashboard.available') }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Uploads Section -->
      <div
        class="accordion-item"
        :class="{ 'is-active': activeTab === 'uploads' }"
      >
        <button class="accordion-header" @click="toggleTab('uploads')">
          <ChevronDown :size="14" class="chevron" />
          <FileUp :size="14" class="section-icon" />
          <span class="section-title">{{ t('dashboard.uploads') }}</span>
          <span
            v-if="uploadTasksData.length > 0 && activeTab !== 'uploads'"
            class="header-badge uploads"
          >
            {{ uploadTasksData.length }}
          </span>
        </button>

        <div
          v-if="activeTab === 'uploads'"
          class="accordion-content uploads-content"
        >
          <div v-if="uploadTasksData.length === 0" class="empty-state">
            <Activity :size="24" />
            <p>{{ t('dashboard.noUploadTasks') }}</p>
          </div>
          <div v-else class="tasks-list">
            <div
              v-for="task in uploadTasksData"
              :key="task.id"
              class="task-item"
              :class="{
                success: task.status === 'success',
                error: task.status === 'error',
                uploading: task.status === 'uploading',
                pending: task.status === 'pending',
              }"
            >
              <div class="task-header">
                <div class="task-info">
                  <component
                    :is="getTaskStatusIcon(task.status)"
                    :size="14"
                    class="task-icon"
                    :style="{ color: getTaskStatusColor(task.status) }"
                  />
                  <div class="task-name-group">
                    <span class="task-name" :title="task.fileName">{{
                      task.fileName
                    }}</span>
                    <span
                      v-if="task.remotePath"
                      class="task-path"
                      :title="task.remotePath"
                    >
                      {{ t('dashboard.to') }}: {{ task.remotePath }}
                    </span>
                  </div>
                </div>
                <span class="task-time">{{
                  formatTaskTime(task.timestamp)
                }}</span>
              </div>
              <div class="progress-container">
                <div class="progress-bar">
                  <div
                    class="progress-fill"
                    :class="`progress-fill-${task.status}`"
                    :style="{ width: `${task.progress}%` }"
                  />
                </div>
                <span class="progress-text">{{ task.progress }}%</span>
              </div>
              <div v-if="task.fileSize" class="task-metrics">
                <div class="metric">
                  <HardDrive :size="10" class="metric-icon" />
                  <span class="metric-value">{{
                    formatFileSize(task.fileSize)
                  }}</span>
                </div>
                <div v-if="task.speed" class="metric">
                  <Activity :size="10" class="metric-icon" />
                  <span class="metric-value">{{
                    formatSpeed(task.speed)
                  }}</span>
                </div>
                <div
                  v-if="task.status === 'uploading' && task.eta"
                  class="metric"
                >
                  <Clock :size="10" class="metric-icon" />
                  <span class="metric-value">{{
                    formatETASeconds(task.eta)
                  }}</span>
                </div>
              </div>
              <p
                v-if="task.status === 'error' && task.error"
                class="task-error"
              >
                {{ task.error }}
              </p>
              <p v-else-if="task.message" class="task-message">
                {{ task.message }}
              </p>
            </div>
            <button
              v-if="uploadTasksData.length > 0"
              class="clear-btn"
              @click="emit('clear-tasks')"
            >
              <Trash2 :size="14" />
              <span>{{ t('dashboard.clearAll') }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped src="@/styles/components/server-dashboard.css"></style>
