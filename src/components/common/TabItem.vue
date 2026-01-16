<script setup lang="ts">
import { Home } from 'lucide-vue-next';
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
    }"
    :data-id="id"
    @click="handleClick"
  >
    <Home v-if="type === 'home'" class="home-icon" :size="14" />
    <span class="tab-label">{{ label }}</span>
    <button
      v-if="closable"
      class="close-btn"
      aria-label="Close tab"
      @click="handleClose"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path
          d="M2,2 L10,10 M10,2 L2,10"
          stroke="currentColor"
          stroke-width="1.5"
        />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.tab-item {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 28px;
  margin: 4px 2px;
  padding: 0 10px;
  background-color: transparent;
  border: none;
  min-width: 110px;
  max-width: 180px;
  border-radius: var(--radius-md);
  position: relative;
  transition: all var(--transition-base);
  user-select: none;
}

.tab-item:hover {
  background-color: var(--color-bg-hover);
}

.tab-item.active {
  background-color: var(--color-bg-primary);
  box-shadow:
    0 4px 12px rgba(0, 0, 0, 0.12),
    0 0 0 1px var(--color-border-primary);
  transform: translateY(-1px);
}

.tab-item.active .tab-label {
  color: var(--color-text-primary);
  font-weight: 600;
}

/* Add a subtle accent pill on the left of active tab */
.tab-item.active::before {
  content: '';
  position: absolute;
  left: 6px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 14px;
  background-color: var(--color-primary);
  border-radius: 100px;
  box-shadow: 0 0 8px var(--color-primary);
}

.tab-item.active.home-tab::before {
  display: none; /* Icon handles the intent for home tab */
}

/* Home tab special styling (Overrides general active styles) */
.home-tab {
  background-color: rgba(0, 122, 255, 0.05);
  border: 1px solid rgba(0, 122, 255, 0.1);
}

.home-tab .tab-label {
  font-weight: 700;
  letter-spacing: 0.5px;
  color: var(--color-primary);
}

.home-tab.active {
  background-color: var(--color-primary) !important;
  border-color: var(--color-primary) !important;
  box-shadow: 0 4px 12px rgba(0, 122, 255, 0.3);
}

.home-tab.active .tab-label,
.home-tab.active .home-icon {
  color: white !important;
}

.home-icon {
  margin-right: -2px;
  color: var(--color-primary);
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  background-color: transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  transition: all var(--transition-base);
  flex-shrink: 0;
  opacity: 0;
}

.tab-item:hover .close-btn,
.tab-item.active .close-btn {
  opacity: 0.6;
}

.close-btn:hover {
  background-color: var(--color-interactive-hover);
  color: var(--color-text-primary);
  opacity: 1 !important;
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
