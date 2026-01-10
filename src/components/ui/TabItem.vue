<script setup lang="ts">
interface Props {
  id: string;
  label: string;
  active?: boolean;
  closable?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  active: false,
  closable: true
});

const emit = defineEmits<{
  'click': [id: string];
  'close': [id: string];
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
    class="tab-item interactive" 
    :class="{ active }"
    @click="handleClick"
  >
    <span class="tab-label">{{ label }}</span>
    <button 
      v-if="closable"
      class="close-btn"
      @click="handleClose"
      aria-label="Close tab"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path d="M2,2 L10,10 M10,2 L2,10" stroke="currentColor" stroke-width="1.5" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.tab-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: calc(100% - 12px);
  margin-top: 6px;
  margin-bottom: 6px;
  padding: 0 14px;
  background-color: transparent;
  border: 0.5px solid var(--color-border-tertiary);
  border-bottom: none;
  min-width: 120px;
  max-width: 220px;
  border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  margin-left: 2px;
  margin-right: 2px;
  transition: all var(--transition-base);
}

.tab-item:hover {
  background-color: var(--color-interactive-hover);
  border-color: var(--color-border-secondary);
}

.tab-item.active {
  background-color: var(--color-bg-primary);
  border-color: var(--color-border-secondary);
  border-bottom: 0.5px solid transparent;
  box-shadow: var(--shadow-sm);
}

.tab-label {
  flex: 1;
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-item.active .tab-label {
  color: var(--color-text-primary);
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  background-color: transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  transition: all var(--transition-base);
  flex-shrink: 0;
  opacity: 0.6;
}

.tab-item:hover .close-btn {
  opacity: 1;
}

.close-btn:hover {
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .tab-item.active {
    background-color: var(--color-bg-primary);
    border-color: var(--color-border-secondary);
    border-bottom: 0.5px solid transparent;
    box-shadow: var(--shadow-md);
  }
}

:root.theme-dark .tab-item.active {
  background-color: var(--color-bg-primary);
  border-color: var(--color-border-secondary);
  border-bottom: 0.5px solid transparent;
  box-shadow: var(--shadow-md);
}
</style>
