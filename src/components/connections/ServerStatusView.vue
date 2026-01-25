<script setup lang="ts">
import { 
  Activity, 
  ArrowDown, 
  ArrowUp, 
  Cpu, 
  MemoryStick, 
  HardDrive, 
  Network 
} from 'lucide-vue-next';

/**
 * ServerStatusView.vue
 * A floating status bar at the top center of the terminal view.
 * Displays connection info and performance metrics.
 */
interface Props {
  sessionId?: string;
}

defineProps<Props>();
</script>

<template>
  <div class="server-status-view">
    <div class="status-item metrics">
      <Network :size="12" class="icon small" />
      <div class="metric">
        <Activity :size="12" class="icon-metric" />
        <span class="value">24ms</span>
      </div>
      <div class="metric">
        <ArrowDown :size="12" class="icon-metric" />
        <span class="value">1.2 KB/s</span>
      </div>
      <div class="metric">
        <ArrowUp :size="12" class="icon-metric" />
        <span class="value">0.5 KB/s</span>
      </div>
    </div>

    <div class="divider"></div>

    <div class="status-item resources">
      <div class="resource-pill">
        <Cpu :size="12" class="icon small" />
        <span class="label">CPU</span>
        <div class="progress-bg">
          <div class="progress-bar" style="width: 15%"></div>
        </div>
      </div>
      <div class="resource-pill">
        <MemoryStick :size="12" class="icon small" />
        <span class="label">MEM</span>
        <div class="progress-bg">
          <div class="progress-bar" style="width: 42%"></div>
        </div>
      </div>
      <div class="resource-pill">
        <HardDrive :size="12" class="icon small" />
        <span class="label">DISK</span>
        <div class="progress-bg">
          <div class="progress-bar" style="width: 28%"></div>
        </div>
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
  color: #BBB;
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
}
</style>
