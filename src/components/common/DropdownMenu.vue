<script setup lang="ts">
import {
  ref,
  computed,
  onMounted,
  onUnmounted,
  watch,
  CSSProperties,
  nextTick,
} from 'vue';
import { ChevronRight } from 'lucide-vue-next';

interface MenuItem {
  key: string;
  label: string;
  danger?: boolean;
  active?: boolean;
  disabled?: boolean;
  divider?: boolean;
  shortcut?: string;
  icon?: any;
  children?: MenuItem[];
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
const activeSubMenu = ref<string | null>(null);
const adjustedX = ref(props.x);
const adjustedY = ref(props.y);
const isPositioned = ref(false);
const submenuDirections = ref<Record<string, 'left' | 'right'>>({});

const menuStyle = computed<CSSProperties>(() => {
  if (props.visible) {
    return {
      position: 'fixed',
      left: `${adjustedX.value}px`,
      top: `${adjustedY.value}px`,
      zIndex: 9998,
      // Initially hide during position calculation to avoid flicker
      opacity: isPositioned.value ? 1 : 0,
    };
  }
  return {};
});

const updatePosition = async () => {
  if (!props.visible) return;

  // Wait for DOM to be ready
  if (!menuRef.value) {
    await nextTick();
  }

  if (menuRef.value) {
    const rect = menuRef.value.getBoundingClientRect();

    // If rect is 0 (not in DOM yet), try one more time
    if (rect.width === 0) {
      await new Promise(resolve => setTimeout(resolve, 0));
    }

    const finalRect = menuRef.value.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    let nextX = props.x;
    let nextY = props.y;

    // Check right boundary
    if (props.x + finalRect.width > viewportWidth) {
      nextX = viewportWidth - finalRect.width - 8;
    }

    // Check bottom boundary
    if (props.y + finalRect.height > viewportHeight) {
      nextY = viewportHeight - finalRect.height - 8;
    }

    // Ensure it doesn't go off the top or left
    nextX = Math.max(8, nextX);
    nextY = Math.max(8, nextY);

    adjustedX.value = nextX;
    adjustedY.value = nextY;
  }

  // Always set isPositioned to true if we are visible,
  // even if calculation didn't perfect, to ensure it shows up.
  isPositioned.value = true;
};

const handleItemClick = (item: MenuItem) => {
  if (item.disabled || item.children) return;
  emit('select', item.key);
  emit('update:visible', false);
  activeSubMenu.value = null;
};

const handleMouseEnterItem = (item: MenuItem, event: MouseEvent) => {
  if (item.children) {
    activeSubMenu.value = item.key;

    // Determine submenu direction
    nextTick(() => {
      const parentItem = event.currentTarget as HTMLElement;
      const submenu = parentItem.querySelector('.submenu') as HTMLElement;
      if (submenu) {
        const rect = submenu.getBoundingClientRect();
        if (rect.right > window.innerWidth) {
          submenuDirections.value[item.key] = 'left';
        } else {
          submenuDirections.value[item.key] = 'right';
        }
      }
    });
  } else {
    activeSubMenu.value = null;
  }
};

const handleClickOutside = (event: MouseEvent) => {
  if (menuRef.value && !menuRef.value.contains(event.target as Node)) {
    emit('update:visible', false);
    activeSubMenu.value = null;
  }
};

const handleMouseEnterMenu = () => {
  clearTimeout((window as any).dropdownHideTimeout);
};

const handleMouseLeaveMenu = () => {
  (window as any).dropdownHideTimeout = setTimeout(() => {
    emit('update:visible', false);
    activeSubMenu.value = null;
  }, 500);
};

const handleVisibleChange = (newVal: boolean) => {
  if (newVal) {
    isPositioned.value = false;
    adjustedX.value = props.x;
    adjustedY.value = props.y;
    // Calculation internally handles wait for DOM
    updatePosition();
    setTimeout(() => {
      document.addEventListener('click', handleClickOutside);
    }, 0);
  } else {
    document.removeEventListener('click', handleClickOutside);
    activeSubMenu.value = null;
    isPositioned.value = false;
    if ((window as any).dropdownHideTimeout) {
      clearTimeout((window as any).dropdownHideTimeout);
    }
  }
};

watch(() => props.visible, handleVisibleChange);

watch([() => props.x, () => props.y], () => {
  if (props.visible) {
    updatePosition();
  }
});

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
                'menu-item-active': item.active,
                'has-submenu': item.children,
                disabled: item.disabled,
              }"
              @click="handleItemClick(item)"
              @mouseenter="handleMouseEnterItem(item, $event)"
            >
              <div class="menu-item-content">
                <component
                  :is="item.icon"
                  v-if="item.icon"
                  class="menu-icon"
                  :size="16"
                />
                <span class="menu-label">{{ item.label }}</span>
              </div>
              <div class="menu-item-suffix">
                <span v-if="item.shortcut" class="menu-shortcut">{{
                  item.shortcut
                }}</span>
                <ChevronRight
                  v-if="item.children"
                  class="submenu-chevron"
                  :size="14"
                />
              </div>

              <!-- Submenu -->
              <div
                v-if="item.children && activeSubMenu === item.key"
                class="submenu panel"
                :class="{
                  'submenu-left': submenuDirections[item.key] === 'left',
                }"
              >
                <div class="menu-list">
                  <template v-for="child in item.children" :key="child.key">
                    <div v-if="child.divider" class="menu-divider" />
                    <div
                      v-else
                      class="menu-item"
                      :class="{
                        'menu-item-danger': child.danger,
                        'menu-item-active': child.active,
                        disabled: child.disabled,
                      }"
                      @click.stop="handleItemClick(child)"
                    >
                      <div class="menu-item-content">
                        <component
                          :is="child.icon"
                          v-if="child.icon"
                          class="menu-icon"
                          :size="16"
                        />
                        <span class="menu-label">{{ child.label }}</span>
                      </div>
                      <span v-if="child.shortcut" class="menu-shortcut">{{
                        child.shortcut
                      }}</span>
                    </div>
                  </template>
                </div>
              </div>
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
  /* Override .panel's overflow: hidden to allow submenus to be visible */
  overflow: visible !important;
}

.menu-list {
  padding: 8px;
  position: relative;
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
  gap: 12px;
  position: relative;
}

.menu-item-content {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.menu-icon {
  color: var(--color-text-secondary);
  flex-shrink: 0;
}

.menu-item:hover .menu-icon {
  color: var(--color-primary);
}

.menu-item:hover {
  background-color: var(--color-interactive-hover);
}

.menu-label {
  flex: 1;
}

.menu-item-suffix {
  display: flex;
  align-items: center;
  gap: 8px;
}

.menu-shortcut {
  font-size: 11px;
  font-family:
    ui-monospace, 'SF Mono', 'Cascadia Mono', 'Consolas', Monaco, 'Courier New',
    monospace;
  background-color: var(--color-bg-tertiary);
  padding: 3px 7px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
}

.submenu-chevron {
  color: var(--color-text-tertiary);
}

.menu-item:hover .menu-shortcut {
  background-color: var(--color-bg-elevated);
}

.submenu {
  position: absolute;
  left: 100%;
  top: -8px;
  min-width: 180px;
  z-index: 9999;
  margin-left: 4px;
}

.submenu-left {
  left: auto;
  right: 100%;
  margin-left: 0;
  margin-right: 4px;
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

.menu-item-active {
  color: var(--color-primary);
  background-color: var(--color-bg-tertiary);
  font-weight: 500;
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
