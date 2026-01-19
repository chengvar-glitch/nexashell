<script setup lang="ts">
import { Home, Terminal, Server } from 'lucide-vue-next';
import { type TabType } from '@/features/tabs';

interface Props {
  id: string;
  label: string;
  active?: boolean;
  closable?: boolean;
  type?: TabType;
}

const props = withDefaults(defineProps<Props>(), {
  active: false,
  closable: true,
  type: 'terminal',
});

const emit = defineEmits<{
  click: [id: string];
  close: [id: string];
}>();

const handleClick = () => {
  emit('click', props.id);
};

const handleClose = (e: Event) => {
  e.stopPropagation();
  if (props.closable) {
    emit('close', props.id);
  }
};
</script>

<template>
  <div
    class="tab-item interactive no-drag"
    :class="{
      active,
      'home-tab': type === 'home',
      'terminal-tab': type === 'terminal',
      'ssh-tab': type === 'ssh',
    }"
    :data-id="id"
    @click="handleClick"
  >
    <Home v-if="type === 'home'" class="tab-icon home-icon" :size="14" />
    <Terminal
      v-else-if="type === 'terminal'"
      class="tab-icon terminal-icon"
      :size="14"
    />
    <Server v-else-if="type === 'ssh'" class="tab-icon ssh-icon" :size="14" />
    <span class="tab-label">{{ label }}</span>
    <div v-if="closable" class="close-btn-wrapper">
      <button class="close-btn" aria-label="Close tab" @click="handleClose">
        <svg width="10" height="10" viewBox="0 0 12 12">
          <path
            d="M2,2 L10,10 M10,2 L2,10"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  margin: 4px 2px;
  padding: 0 8px 0 12px;
  background-color: transparent;
  border: 1px solid transparent;
  min-width: 120px;
  max-width: 200px;
  border-radius: var(--radius-md);
  position: relative;
  transition: all var(--transition-base);
  user-select: none;
}

.tab-item:hover {
  background-color: var(--color-interactive-hover);
}

.tab-item.active {
  background-color: var(--color-bg-primary);
  border-color: var(--color-border-primary);
  box-shadow: var(--shadow-sm);
}

.tab-item.active .tab-label {
  color: var(--color-primary);
  font-weight: 500;
}

/* Remove the old accent pill */

.tab-label {
  flex: 1;
  font-size: 13px;
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.close-btn-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 100%;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  padding: 0;
  border: none;
  background-color: transparent;
  border-radius: var(--radius-xs);
  cursor: pointer;
  color: var(--color-text-tertiary);
  transition: all var(--transition-fast);
  flex-shrink: 0;
  opacity: 0;
}

.tab-item:hover .close-btn,
.tab-item.active .close-btn {
  opacity: 0.8;
}

.close-btn:hover {
  background-color: rgba(255, 95, 87, 0.1);
  color: var(--color-macos-close);
  opacity: 1 !important;
}

/* Home tab special styling */
.home-tab.active {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
}

.home-tab.active .tab-label,
.home-tab.active .home-icon {
  color: white;
}

.home-tab.active .close-btn {
  color: rgba(255, 255, 255, 0.7);
}

.home-tab.active .close-btn:hover {
  background-color: rgba(255, 255, 255, 0.2);
  color: white;
}

.tab-icon {
  flex-shrink: 0;
  opacity: 0.7;
}

.tab-item:hover .tab-icon,
.tab-item.active .tab-icon {
  opacity: 1;
}

.home-icon {
  color: var(--color-primary);
}

.terminal-icon {
  color: var(--color-text-secondary);
}

.ssh-icon {
  color: var(--color-primary);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .tab-item.active {
    background-color: var(--color-bg-elevated);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  }
}

:root.theme-dark .tab-item.active {
  background-color: var(--color-bg-elevated);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}
</style>
