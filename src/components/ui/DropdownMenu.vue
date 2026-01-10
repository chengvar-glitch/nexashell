<script setup lang="ts">
import {
  ref,
  computed,
  onMounted,
  onUnmounted,
  watch,
  CSSProperties,
} from 'vue';

interface MenuItem {
  key: string;
  label: string;
  danger?: boolean;
  disabled?: boolean;
  divider?: boolean;
  shortcut?: string;
}

interface Props {
  items: MenuItem[];
  visible?: boolean;
  x?: number;
  y?: number;
  trigger?: 'click' | 'contextmenu';
}

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  x: 0,
  y: 0,
  trigger: 'click',
});

const emit = defineEmits<{
  'update:visible': [value: boolean];
  select: [key: string];
}>();

const menuRef = ref<HTMLElement | null>(null);
const menuStyle = computed<CSSProperties>(() => {
  if (props.x || props.y) {
    return {
      position: 'fixed',
      left: `${props.x}px`,
      top: `${props.y}px`,
      zIndex: 9998,
    };
  }
  return {};
});

const handleItemClick = (item: MenuItem) => {
  if (item.disabled) return;
  emit('select', item.key);
  emit('update:visible', false);
};

const handleClickOutside = (event: MouseEvent) => {
  if (menuRef.value && !menuRef.value.contains(event.target as Node)) {
    emit('update:visible', false);
  }
};

const handleMouseEnterMenu = () => {
  clearTimeout((window as any).dropdownHideTimeout);
};
const handleMouseLeaveMenu = () => {
  (window as any).dropdownHideTimeout = setTimeout(() => {
    emit('update:visible', false);
  }, 500);
};

const handleVisibleChange = (newVal: boolean) => {
  if (newVal) {
    setTimeout(() => {
      document.addEventListener('click', handleClickOutside);
    }, 0);
  } else {
    document.removeEventListener('click', handleClickOutside);
    if ((window as any).dropdownHideTimeout) {
      clearTimeout((window as any).dropdownHideTimeout);
    }
  }
};

watch(() => props.visible, handleVisibleChange);

onMounted(() => {
  if (props.visible) {
    document.addEventListener('click', handleClickOutside);
  }
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  if ((window as any).dropdownHideTimeout) {
    clearTimeout((window as any).dropdownHideTimeout);
  }
});
</script>

<template>
  <Teleport to="body">
    <Transition name="dropdown-fade">
      <div
        v-if="visible"
        ref="menuRef"
        class="dropdown-menu panel"
        :style="menuStyle"
        @mouseenter="handleMouseEnterMenu"
        @mouseleave="handleMouseLeaveMenu"
      >
        <div class="menu-list">
          <template v-for="item in items" :key="item.key">
            <div v-if="item.divider" class="menu-divider" />
            <div
              v-else
              class="menu-item"
              :class="{
                'menu-item-danger': item.danger,
                disabled: item.disabled,
              }"
              @click="handleItemClick(item)"
            >
              <span class="menu-label">{{ item.label }}</span>
              <span v-if="item.shortcut" class="menu-shortcut">{{
                item.shortcut
              }}</span>
            </div>
          </template>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.dropdown-menu {
  position: absolute;
  min-width: 200px;
  overflow: hidden;
}

.menu-list {
  padding: 8px;
}

.menu-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--color-text-primary);
  cursor: pointer;
  transition: all var(--transition-base);
  border-radius: var(--radius-md);
}

.menu-item:hover {
  background-color: var(--color-interactive-hover);
}

.menu-label {
  flex: 1;
}

.menu-shortcut {
  margin-left: 20px;
  font-size: 11px;
  font-family:
    ui-monospace, 'SF Mono', 'Cascadia Mono', 'Consolas', Monaco, 'Courier New',
    monospace;
  background-color: var(--color-bg-tertiary);
  padding: 3px 7px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
}

.menu-item:hover .menu-shortcut {
  background-color: var(--color-bg-elevated);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .menu-item:hover .menu-shortcut {
    background-color: var(--color-bg-elevated);
  }
}

:root.theme-dark .menu-item:hover .menu-shortcut {
  background-color: var(--color-bg-elevated);
}

.menu-item-danger {
  color: #ff3b30;
}

.menu-item-danger:hover {
  background-color: rgba(255, 59, 48, 0.08);
}

.menu-divider {
  height: 0.5px;
  margin: 8px 0;
  background-color: var(--color-border-secondary);
}

.dropdown-fade-enter-active,
.dropdown-fade-leave-active {
  transition:
    opacity var(--transition-base),
    transform var(--transition-base);
}

.dropdown-fade-enter-from {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}

.dropdown-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .menu-item-danger {
    color: #ff453a;
  }

  :root:not(.theme-light) .menu-item-danger:hover {
    background-color: rgba(255, 69, 58, 0.12);
  }
}

:root.theme-dark .menu-item-danger {
  color: #ff453a;
}

:root.theme-dark .menu-item-danger:hover {
  background-color: rgba(255, 69, 58, 0.12);
}
</style>
