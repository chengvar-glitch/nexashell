<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';

interface Props {
  text: string;
  position?: 'top' | 'bottom' | 'left' | 'right';
  offset?: number;
}

const props = withDefaults(defineProps<Props>(), {
  position: 'top',
  offset: 8
});

const tooltipRef = ref<HTMLElement | null>(null);
const tooltipPosition = ref({ top: '0px', left: '0px' });
const updatePosition = () => {
  if (!tooltipRef.value) return;
  
  const parentElement = tooltipRef.value.parentElement;
  if (!parentElement) return;
  
  const rect = parentElement.getBoundingClientRect();
  const tooltipRect = tooltipRef.value.getBoundingClientRect();
  
  let top = 0;
  let left = 0;
  
  switch (props.position) {
    case 'top':
      top = rect.top - tooltipRect.height - props.offset;
      left = rect.left + (rect.width - tooltipRect.width) / 2;
      break;
    case 'bottom':
      top = rect.bottom + props.offset;
      left = rect.left + (rect.width - tooltipRect.width) / 2;
      break;
    case 'left':
      top = rect.top + (rect.height - tooltipRect.height) / 2;
      left = rect.left - tooltipRect.width - props.offset;
      break;
    case 'right':
      top = rect.top + (rect.height - tooltipRect.height) / 2;
      left = rect.right + props.offset;
      break;
  }
  
  tooltipPosition.value = {
    top: `${top}px`,
    left: `${left}px`
  };
};

onMounted(() => {
  window.addEventListener('resize', updatePosition);
  setTimeout(updatePosition, 0);
});

onUnmounted(() => {
  window.removeEventListener('resize', updatePosition);
});
</script>

<template>
  <div class="shortcut-hint-wrapper">
    <slot></slot>
    <div 
      ref="tooltipRef"
      class="shortcut-hint"
      :style="{ ...tooltipPosition }"
    >
      <span class="shortcut-text">{{ text }}</span>
    </div>
  </div>
</template>

<style scoped>
.shortcut-hint-wrapper {
  position: relative;
  display: inline-block;
}

.shortcut-hint {
  position: fixed;
  background-color: var(--color-bg-elevated);
  color: var(--color-text-primary);
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  white-space: nowrap;
  z-index: 1000;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
  border: 0.5px solid var(--color-border-secondary);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .shortcut-hint {
    background-color: var(--color-bg-elevated);
    color: var(--color-text-primary);
  }
}

:root.theme-dark .shortcut-hint {
  background-color: var(--color-bg-elevated);
  color: var(--color-text-primary);
}
</style>