<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as echarts from 'echarts';
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

const cpuChartRef = ref<HTMLElement | null>(null);
const memChartRef = ref<HTMLElement | null>(null);
const netChartRef = ref<HTMLElement | null>(null);

let cpuChart: echarts.ECharts | null = null;
let memChart: echarts.ECharts | null = null;
let netChart: echarts.ECharts | null = null;

let unlisten: UnlistenFn | null = null;

const initCharts = () => {
  if (cpuChartRef.value) {
    cpuChart = echarts.init(cpuChartRef.value, 'dark');
    cpuChart.setOption(getChartOption('#3b82f6', '%'));
  }
  if (memChartRef.value) {
    memChart = echarts.init(memChartRef.value, 'dark');
    memChart.setOption(getChartOption('#10b981', '%'));
  }
  if (netChartRef.value) {
    netChart = echarts.init(netChartRef.value, 'dark');
    netChart.setOption({
      ...getChartOption('#3b82f6', ' KB/s'),
      series: [
        {
          name: 'Down',
          type: 'line',
          smooth: true,
          showSymbol: false,
          areaStyle: { opacity: 0.1 },
          lineStyle: { width: 1.5, color: '#3b82f6' },
          data: [],
        },
        {
          name: 'Up',
          type: 'line',
          smooth: true,
          showSymbol: false,
          areaStyle: { opacity: 0.1 },
          lineStyle: { width: 1.5, color: '#f59e0b' },
          data: [],
        },
      ],
    });
  }
};

const getChartOption = (color: string, unit: string) => ({
  backgroundColor: 'transparent',
  tooltip: {
    trigger: 'axis',
    backgroundColor: 'rgba(20, 20, 20, 0.9)',
    borderSize: 0,
    borderWidth: 0,
    textStyle: { color: '#fff', fontSize: 10 },
    padding: [4, 8],
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    formatter: (params: any) => {
      const p = params[0];
      return `${p.value.toFixed(1)}${unit}`;
    },
  },
  grid: { left: 4, right: 4, bottom: 4, top: 10, containLabel: false },
  xAxis: {
    type: 'category',
    boundaryGap: false,
    data: Array(MAX_HISTORY).fill(''),
    axisLine: { show: false },
    axisTick: { show: false },
    axisLabel: { show: false },
  },
  yAxis: {
    type: 'value',
    min: 0,
    max: unit === '%' ? 100 : undefined,
    splitLine: {
      show: true,
      lineStyle: { color: 'rgba(255,255,255,0.05)', type: 'dashed' },
    },
    axisLine: { show: false },
    axisTick: { show: false },
    axisLabel: { show: false },
  },
  series: [
    {
      type: 'line',
      smooth: true,
      showSymbol: false,
      areaStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: color + '44' },
          { offset: 1, color: color + '00' },
        ]),
        opacity: 0.3,
      },
      lineStyle: { width: 2, color: color },
      data: [],
    },
  ],
});

const updateCharts = () => {
  if (!cpuChart || !memChart || !netChart) return;

  const padData = (data: number[]) => {
    const padded = new Array(MAX_HISTORY).fill(null);
    const startIdx = Math.max(0, MAX_HISTORY - data.length);
    for (let i = 0; i < data.length; i++) {
      if (startIdx + i < MAX_HISTORY) {
        padded[startIdx + i] = data[i];
      }
    }
    return padded;
  };

  const cpuData = padData(historyData.value.map(h => h.cpuUsage));
  const memData = padData(historyData.value.map(h => h.memUsage));
  const netDownData = padData(historyData.value.map(h => h.netDown / 1024)); // KB/s
  const netUpData = padData(historyData.value.map(h => h.netUp / 1024)); // KB/s

  cpuChart.setOption({ series: [{ data: cpuData }] });
  memChart.setOption({ series: [{ data: memData }] });
  netChart.setOption({
    series: [
      { name: 'Down', data: netDownData, itemStyle: { color: '#3b82f6' } },
      { name: 'Up', data: netUpData, itemStyle: { color: '#f59e0b' } },
    ],
  });
};

const setupListener = async (sid: string) => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }

  if (props.history) return; // Use prop instead

  unlisten = await listen<ServerStatus>(`ssh-status-${sid}`, event => {
    localHistory.value.push(event.payload);
    if (localHistory.value.length > MAX_HISTORY) {
      localHistory.value.shift();
    }
    updateCharts();
  });
};

onMounted(async () => {
  await nextTick();
  initCharts();
  setupListener(props.sessionId);
  updateCharts();

  const handleResize = () => {
    cpuChart?.resize();
    memChart?.resize();
    netChart?.resize();
  };
  window.addEventListener('resize', handleResize);
  onUnmounted(() => window.removeEventListener('resize', handleResize));
});

onUnmounted(() => {
  if (unlisten) unlisten();
  cpuChart?.dispose();
  memChart?.dispose();
  netChart?.dispose();
});

watch(
  () => historyData.value,
  () => {
    updateCharts();
  },
  { deep: true }
);

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
        <div ref="cpuChartRef" class="chart"></div>
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
        <div ref="memChartRef" class="chart"></div>
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
        <div ref="netChartRef" class="chart"></div>
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

.chart {
  width: 100%;
  height: 80px;
  margin-top: 4px;
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
