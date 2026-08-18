<script setup lang="ts">
import { ref } from 'vue';
import { Home, Terminal, Server } from 'lucide-vue-next';
import { type TabType } from '@/features/tabs';
import { useI18n } from 'vue-i18n';
import DropdownMenu from '@/components/common/DropdownMenu.vue';
import { eventBus } from '@/core/utils/event-bus';
import { APP_EVENTS } from '@/core/constants';

const { t } = useI18n({ useScope: 'global' });

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

const contextMenuVisible = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);

const handleClick = () => {
  emit('click', props.id);
};

const handleClose = (e: Event) => {
  e.stopPropagation();
  if (props.closable) {
    emit('close', props.id);
  }
};

const handleContextMenu = (e: MouseEvent) => {
  if (props.type === 'home') return;
  e.preventDefault();
  e.stopPropagation();
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  contextMenuVisible.value = true;
};

const handleContextMenuSelect = (key: string) => {
  contextMenuVisible.value = false;
  if (key === 'split-horizontal' || key === 'split-vertical') {
    // Activate this tab FIRST so splitActivePane operates on the
    // right-clicked tab (setActiveTab syncs activePaneId to its first pane)
    emit('click', props.id);
    if (key === 'split-horizontal') {
      eventBus.emit(APP_EVENTS.SPLIT_HORIZONTAL);
    } else {
      eventBus.emit(APP_EVENTS.SPLIT_VERTICAL);
    }
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
    @contextmenu="handleContextMenu"
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
      <button class="close-btn" :aria-label="t('common.closeTab')" @click="handleClose">
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

    <DropdownMenu
      v-model:visible="contextMenuVisible"
      :x="contextMenuX"
      :y="contextMenuY"
      :trigger="'contextmenu'"
      :items="[
        { key: 'split-vertical', label: t('pane.splitVertical') },
        { key: 'split-horizontal', label: t('pane.splitHorizontal') },
      ]"
      @select="handleContextMenuSelect"
    />
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
  transition: background-color var(--transition-fast), border-color var(--transition-fast), box-shadow var(--transition-fast), transform var(--transition-micro);
  user-select: none;
  will-change: transform;
}

.tab-item:hover {
  background-color: var(--color-interactive-hover);
}

.tab-item:active {
  transform: scale(0.96);
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
  transition: transform var(--transition-micro), background-color var(--transition-fast), color var(--transition-fast), opacity var(--transition-fast);
  flex-shrink: 0;
  opacity: 0;
  will-change: transform;
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

.close-btn:active {
  transform: scale(0.8);
  background-color: rgba(255, 95, 87, 0.2);
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
