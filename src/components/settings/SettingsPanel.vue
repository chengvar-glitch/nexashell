<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Settings, Palette, Terminal, Keyboard, Info } from 'lucide-vue-next';
import { themeManager, type ThemeMode } from '@/utils/theme-manager';

interface Props {
  visible?: boolean;
}

withDefaults(defineProps<Props>(), {
  visible: false,
});

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const activeMenu = ref('general');
const selectedTheme = ref<ThemeMode>('auto');

const menuItems = [
  { key: 'general', label: 'General', icon: Settings },
  { key: 'appearance', label: 'Appearance', icon: Palette },
  { key: 'terminal', label: 'Terminal', icon: Terminal },
  { key: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
  { key: 'about', label: 'About', icon: Info },
];

const handleClose = () => {
  emit('update:visible', false);
};

const handleMenuClick = (key: string) => {
  activeMenu.value = key;
};

const handleThemeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  const theme = target.value as ThemeMode;
  selectedTheme.value = theme;
  themeManager.setTheme(theme);
};

onMounted(() => {
  selectedTheme.value = themeManager.getTheme();
});
</script>

<template>
  <Teleport to="body">
    <Transition name="settings-fade">
      <div
        v-if="visible"
        class="settings-overlay flex-center"
        @click="handleClose"
      >
        <div
          class="settings-panel panel"
          @click.stop
        >
          <div class="settings-header border-bottom draggable">
            <div class="macos-controls no-drag">
              <button
                class="control-btn close"
                aria-label="Close"
                @click="handleClose"
              />
            </div>
            <h2 class="settings-title">
              Settings
            </h2>
          </div>

          <div class="settings-body">
            <div class="settings-sidebar border-right">
              <nav class="settings-menu">
                <button
                  v-for="item in menuItems"
                  :key="item.key"
                  class="menu-item"
                  :class="{ active: activeMenu === item.key }"
                  @click="handleMenuClick(item.key)"
                >
                  <component
                    :is="item.icon"
                    :size="16"
                    class="menu-icon"
                  />
                  <span class="menu-label">{{ item.label }}</span>
                </button>
              </nav>
            </div>

            <div class="settings-content">
              <div
                v-if="activeMenu === 'general'"
                class="content-section"
              >
                <h3 class="section-title">
                  General Settings
                </h3>
                <div class="setting-item">
                  <label class="setting-label">Launch on Startup</label>
                  <input
                    type="checkbox"
                    class="setting-checkbox"
                  >
                </div>
                <div class="setting-item">
                  <label class="setting-label">Default Shell</label>
                  <select class="setting-select">
                    <option>Bash</option>
                    <option>Zsh</option>
                    <option>Fish</option>
                  </select>
                </div>
              </div>

              <div
                v-if="activeMenu === 'appearance'"
                class="content-section"
              >
                <h3 class="section-title">
                  Appearance Settings
                </h3>
                <div class="setting-item">
                  <label class="setting-label">Theme</label>
                  <select
                    class="setting-select"
                    :value="selectedTheme"
                    @change="handleThemeChange"
                  >
                    <option value="auto">
                      Auto
                    </option>
                    <option value="light">
                      Light
                    </option>
                    <option value="dark">
                      Dark
                    </option>
                  </select>
                </div>
                <div class="setting-item">
                  <label class="setting-label">Font Size</label>
                  <input
                    type="number"
                    class="setting-input"
                    value="14"
                    min="10"
                    max="24"
                  >
                </div>
              </div>

              <div
                v-if="activeMenu === 'terminal'"
                class="content-section"
              >
                <h3 class="section-title">
                  Terminal Settings
                </h3>
                <div class="setting-item">
                  <label class="setting-label">Cursor Style</label>
                  <select class="setting-select">
                    <option>Block</option>
                    <option>Underline</option>
                    <option>Bar</option>
                  </select>
                </div>
                <div class="setting-item">
                  <label class="setting-label">Enable Cursor Blink</label>
                  <input
                    type="checkbox"
                    class="setting-checkbox"
                    checked
                  >
                </div>
              </div>

              <div
                v-if="activeMenu === 'shortcuts'"
                class="content-section"
              >
                <h3 class="section-title">
                  Keyboard Shortcuts
                </h3>
                <div class="setting-item">
                  <label class="setting-label">New Tab</label>
                  <input
                    type="text"
                    class="setting-input"
                    value="Cmd+T"
                    readonly
                  >
                </div>
                <div class="setting-item">
                  <label class="setting-label">Close Tab</label>
                  <input
                    type="text"
                    class="setting-input"
                    value="Cmd+W"
                    readonly
                  >
                </div>
              </div>

              <div
                v-if="activeMenu === 'about'"
                class="content-section"
              >
                <h3 class="section-title">
                  About
                </h3>
                <div class="about-info">
                  <p><strong>NexaShell</strong></p>
                  <p>Version 1.0.0</p>
                  <p>Modern Terminal Emulator</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-overlay);
  z-index: 1000;
}

.settings-panel {
  width: 760px;
  height: 540px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 24px;
  background-color: var(--color-bg-secondary);
  flex-shrink: 0;
  position: relative;
  height: 48px;
  min-height: 48px;
}

.macos-controls {
  position: absolute;
  left: 16px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  gap: 8px;
}

.control-btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  transition: all var(--transition-base);
  padding: 0;
}

.control-btn.close {
  background-color: var(--color-macos-close);
}

.control-btn.close:hover {
  filter: brightness(0.9);
}

.settings-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
  text-align: center;
  letter-spacing: -0.01em;
}

.settings-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.settings-sidebar {
  width: 220px;
  background-color: var(--color-bg-tertiary);
  padding: 20px 16px;
  flex-shrink: 0;
}

.settings-menu {
  display: flex;
  flex-direction: column;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: none;
  background-color: transparent;
  text-align: left;
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-secondary);
  transition: all var(--transition-base);
  margin-bottom: 2px;
  border-radius: var(--radius-md);
  width: 100%;
}

.menu-icon {
  flex-shrink: 0;
  transition: all 0.15s;
  opacity: 0.7;
}

.menu-label {
  flex: 1;
}

.menu-item:hover {
  background-color: var(--color-interactive-hover);
}

.menu-item.active {
  background-color: var(--color-primary);
  color: #ffffff;
  font-weight: 600;
  box-shadow: var(--shadow-sm);
}

.menu-item.active .menu-icon {
  color: #ffffff;
  opacity: 1;
}

.settings-content {
  flex: 1;
  padding: 24px 28px;
  overflow-y: auto;
  background-color: var(--color-bg-primary);
}

.content-section {
  max-width: 480px;
}

.section-title {
  font-size: 11px;
  font-weight: 700;
  color: var(--color-text-tertiary);
  margin: 0 0 16px 0;
  padding-bottom: 0;
  border-bottom: none;
  text-transform: uppercase;
  letter-spacing: 0.8px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background-color: var(--color-bg-elevated);
  border: 0.5px solid var(--color-border-tertiary);
  border-radius: var(--radius-lg);
  margin-bottom: 10px;
  transition: all var(--transition-base);
}

.setting-item:hover {
  background-color: var(--color-bg-elevated);
  border-color: var(--color-border-secondary);
}

.setting-item:last-child {
  margin-bottom: 0;
}

.setting-label {
  font-size: 13px;
  color: var(--color-text-primary);
  font-weight: 400;
}

.setting-input,
.setting-select {
  padding: 6px 12px;
  border: 0.5px solid var(--color-border-secondary);
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--color-text-primary);
  background-color: var(--color-bg-primary);
  outline: none;
  transition: all var(--transition-base);
}

.setting-input:focus,
.setting-select:focus {
  border-color: var(--color-primary);
  box-shadow: var(--focus-ring);
}

.setting-input {
  width: 160px;
}

.setting-select {
  width: 160px;
}

.setting-checkbox {
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: #4a90e2;
}

.about-info {
  padding: 16px 0;
}

.about-info p {
  margin: 8px 0;
  font-size: 13px;
  color: #666666;
  line-height: 1.5;
}

.about-info strong {
  font-size: 14px;
  color: #000000;
  font-weight: 600;
}

.settings-fade-enter-active,
.settings-fade-leave-active {
  transition: opacity var(--transition-fast);
}

.settings-fade-enter-from,
.settings-fade-leave-to {
  opacity: 0;
}

.settings-fade-enter-active .settings-panel,
.settings-fade-leave-active .settings-panel {
  transition:
    transform var(--transition-base),
    opacity var(--transition-base);
}

.settings-fade-enter-from .settings-panel {
  transform: scale(0.96);
  opacity: 0;
}

.settings-fade-leave-to .settings-panel {
  transform: scale(0.96);
  opacity: 0;
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .menu-item.active {
    background-color: var(--color-primary);
    color: #ffffff;
    box-shadow: var(--shadow-sm);
  }

  :root:not(.theme-light) .setting-checkbox {
    accent-color: var(--color-primary);
  }
}

:root.theme-dark .menu-item.active {
  background-color: var(--color-primary);
  color: #ffffff;
  box-shadow: var(--shadow-sm);
}

:root.theme-dark .setting-checkbox {
  accent-color: var(--color-primary);
}
</style>
