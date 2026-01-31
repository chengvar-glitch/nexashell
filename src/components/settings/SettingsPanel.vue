<template>
  <Teleport v-if="useTeleport" to="body">
    <Transition name="settings-fade">
      <div
        v-if="visible"
        class="settings-overlay flex-center"
        @click="handleClose"
      >
        <div class="settings-panel panel" @click.stop>
          <div class="settings-header border-bottom draggable">
            <div class="macos-controls no-drag">
              <button
                class="control-btn close"
                aria-label="Close"
                @click="handleClose"
              />
            </div>
            <h2 class="settings-title">
              {{ $t('settings.title') }}
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
                  <component :is="item.icon" :size="16" class="menu-icon" />
                  <span class="menu-label">{{ item.label }}</span>
                </button>
              </nav>
            </div>

            <div ref="contentRef" class="settings-content">
              <div
                :ref="el => setSectionRef(el, 'appearance')"
                class="content-section"
              >
                <h3 class="section-title">
                  {{ $t('settings.appearance') }}
                </h3>
                <div class="setting-item">
                  <label class="setting-label">{{
                    $t('settings.theme')
                  }}</label>
                  <select
                    class="setting-select modal-input"
                    :value="selectedTheme"
                    @change="handleThemeChange"
                  >
                    <option value="auto">{{ $t('settings.themeAuto') }}</option>
                    <option value="light">
                      {{ $t('settings.themeLight') }}
                    </option>
                    <option value="dark">{{ $t('settings.themeDark') }}</option>
                  </select>
                </div>
              </div>

              <div
                :ref="el => setSectionRef(el, 'language')"
                class="content-section"
              >
                <h3 class="section-title">
                  {{ $t('settings.language') }}
                </h3>
                <div class="setting-item">
                  <label class="setting-label">{{
                    $t('settings.language')
                  }}</label>
                  <select
                    class="setting-select modal-input"
                    :value="locale"
                    @change="handleLanguageChange"
                  >
                    <option
                      v-for="lang in languages"
                      :key="lang.value"
                      :value="lang.value"
                    >
                      {{ lang.label }}
                    </option>
                  </select>
                </div>
              </div>

              <div
                :ref="el => setSectionRef(el, 'terminal')"
                class="content-section"
              >
                <h3 class="section-title">
                  {{ $t('settings.terminal') }}
                </h3>
                <div class="setting-item">
                  <label class="setting-label">{{
                    $t('settings.cursorStyle')
                  }}</label>
                  <select
                    class="setting-select modal-input"
                    :value="settingsStore.terminal.cursorStyle"
                    @change="handleCursorStyleChange"
                  >
                    <option value="block">
                      {{ $t('settings.cursorBlock') }}
                    </option>
                    <option value="underline">
                      {{ $t('settings.cursorUnderline') }}
                    </option>
                    <option value="bar">{{ $t('settings.cursorBar') }}</option>
                  </select>
                </div>
              </div>

              <div
                :ref="el => setSectionRef(el, 'shortcuts')"
                class="content-section"
              >
                <h3 class="section-title">
                  {{ $t('settings.shortcuts') }}
                </h3>
                <div
                  v-for="shortcut in shortcutList"
                  :key="shortcut.label"
                  class="setting-item"
                >
                  <label class="setting-label">{{ shortcut.label }}</label>
                  <kbd class="shortcut-key">{{ shortcut.value }}</kbd>
                </div>
              </div>

              <div
                :ref="el => setSectionRef(el, 'terminalShortcuts')"
                class="content-section"
              >
                <h3 class="section-title">
                  {{ $t('settings.terminalShortcuts') }}
                </h3>
                <div
                  v-for="shortcut in terminalShortcutList"
                  :key="shortcut.label"
                  class="setting-item"
                >
                  <label class="setting-label">{{ shortcut.label }}</label>
                  <kbd class="shortcut-key">{{ shortcut.value }}</kbd>
                </div>
              </div>

              <div
                :ref="el => setSectionRef(el, 'about')"
                class="content-section"
              >
                <h3 class="section-title">
                  {{ $t('settings.about') }}
                </h3>
                <div class="about-info">
                  <div class="about-header">
                    <div class="about-logo">
                      <img
                        src="/welcome-image.png"
                        alt="NexaShell Logo"
                        class="logo-image"
                      />
                    </div>
                    <div class="about-title-group">
                      <h4 class="about-app-name">NexaShell</h4>
                      <p class="about-version">
                        {{ $t('settings.version') }} {{ appVersion }}
                      </p>
                    </div>
                  </div>

                  <div class="about-content">
                    <p class="about-desc">
                      {{ $t('settings.description') }}
                    </p>

                    <div class="about-meta">
                      <div class="meta-item">
                        <span class="meta-label">{{
                          $t('settings.license')
                        }}</span>
                        <span class="meta-value">MIT</span>
                      </div>
                      <div class="meta-item">
                        <span class="meta-label">{{
                          $t('settings.github')
                        }}</span>
                        <a
                          href="https://github.com/chengvar-glitch/nexashell"
                          target="_blank"
                          class="meta-link"
                        >
                          github.com/chengvar-glitch/nexashell
                        </a>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
  <!-- Render without teleport when useTeleport is false -->
  <Transition v-else name="settings-fade">
    <div v-if="visible" class="modal-system-overlay" @click="handleClose">
      <div class="settings-panel modal-system-panel" @click.stop>
        <div class="settings-header border-bottom draggable">
          <div class="macos-controls no-drag">
            <button
              class="control-btn close"
              aria-label="Close"
              @click="handleClose"
            />
          </div>
          <h2 class="settings-title">
            {{ $t('settings.title') }}
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
                <component :is="item.icon" :size="16" class="menu-icon" />
                <span class="menu-label">{{ item.label }}</span>
              </button>
            </nav>
          </div>

          <div ref="contentRefFallback" class="settings-content">
            <div
              :ref="el => setSectionRef(el, 'appearance')"
              class="content-section"
            >
              <h3 class="section-title">
                {{ $t('settings.appearance') }}
              </h3>
              <div class="setting-item">
                <label class="setting-label">{{ $t('settings.theme') }}</label>
                <select
                  class="setting-select modal-input"
                  :value="selectedTheme"
                  @change="handleThemeChange"
                >
                  <option value="auto">{{ $t('settings.themeAuto') }}</option>
                  <option value="light">{{ $t('settings.themeLight') }}</option>
                  <option value="dark">{{ $t('settings.themeDark') }}</option>
                </select>
              </div>
            </div>

            <div
              :ref="el => setSectionRef(el, 'language')"
              class="content-section"
            >
              <h3 class="section-title">
                {{ $t('settings.language') }}
              </h3>
              <div class="setting-item">
                <label class="setting-label">{{
                  $t('settings.language')
                }}</label>
                <select
                  class="setting-select modal-input"
                  :value="locale"
                  @change="handleLanguageChange"
                >
                  <option
                    v-for="lang in languages"
                    :key="lang.value"
                    :value="lang.value"
                  >
                    {{ lang.label }}
                  </option>
                </select>
              </div>
            </div>

            <div
              :ref="el => setSectionRef(el, 'terminal')"
              class="content-section"
            >
              <h3 class="section-title">
                {{ $t('settings.terminal') }}
              </h3>
              <div class="setting-item">
                <label class="setting-label">{{
                  $t('settings.cursorStyle')
                }}</label>
                <select
                  class="setting-select modal-input"
                  :value="settingsStore.terminal.cursorStyle"
                  @change="handleCursorStyleChange"
                >
                  <option value="block">
                    {{ $t('settings.cursorBlock') }}
                  </option>
                  <option value="underline">
                    {{ $t('settings.cursorUnderline') }}
                  </option>
                  <option value="bar">{{ $t('settings.cursorBar') }}</option>
                </select>
              </div>
            </div>

            <div
              :ref="el => setSectionRef(el, 'shortcuts')"
              class="content-section"
            >
              <h3 class="section-title">
                {{ $t('settings.shortcuts') }}
              </h3>
              <div
                v-for="shortcut in shortcutList"
                :key="shortcut.label"
                class="setting-item"
              >
                <label class="setting-label">{{ shortcut.label }}</label>
                <kbd class="shortcut-key">{{ shortcut.value }}</kbd>
              </div>
            </div>

            <div
              :ref="el => setSectionRef(el, 'terminalShortcuts')"
              class="content-section"
            >
              <h3 class="section-title">
                {{ $t('settings.terminalShortcuts') }}
              </h3>
              <div
                v-for="shortcut in terminalShortcutList"
                :key="shortcut.label"
                class="setting-item"
              >
                <label class="setting-label">{{ shortcut.label }}</label>
                <kbd class="shortcut-key">{{ shortcut.value }}</kbd>
              </div>
            </div>

            <div
              :ref="el => setSectionRef(el, 'about')"
              class="content-section"
            >
              <h3 class="section-title">
                {{ $t('settings.about') }}
              </h3>
              <div class="about-info">
                <div class="about-header">
                  <div class="about-logo">
                    <img
                      src="/welcome-image.png"
                      alt="NexaShell Logo"
                      class="logo-image"
                    />
                  </div>
                  <div class="about-title-group">
                    <h4 class="about-app-name">NexaShell</h4>
                    <p class="about-version">
                      {{ $t('settings.version') }} {{ appVersion }}
                    </p>
                  </div>
                </div>

                <div class="about-content">
                  <p class="about-desc">
                    {{ $t('settings.description') }}
                  </p>

                  <div class="about-meta">
                    <div class="meta-item">
                      <span class="meta-label">{{
                        $t('settings.license')
                      }}</span>
                      <span class="meta-value">MIT</span>
                    </div>
                    <div class="meta-item">
                      <span class="meta-label">{{
                        $t('settings.github')
                      }}</span>
                      <a
                        href="https://github.com/chengvar-glitch/nexashell"
                        target="_blank"
                        class="meta-link"
                      >
                        github.com/chengvar-glitch/nexashell
                      </a>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { getVersion } from '@tauri-apps/api/app';
import {
  Palette,
  Terminal,
  Keyboard,
  Info,
  Languages,
  Zap,
} from 'lucide-vue-next';
import { themeManager, type ThemeMode } from '@/core/utils/theme-manager';
import { useSettingsStore, type CursorStyle } from '@/features/settings';
import { isMacOSBrowser } from '@/core/utils/platform/platform-detection';

interface Props {
  visible?: boolean;
  useTeleport?: boolean; // Add prop to control teleport usage
  initialSection?: string;
}

const { locale, t } = useI18n({ useScope: 'global' });
const settingsStore = useSettingsStore();

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  useTeleport: true, // Default to true for backward compatibility
  initialSection: 'appearance',
});

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const activeMenu = ref('appearance');
const selectedTheme = ref<ThemeMode>('auto');
const appVersion = ref('0.1.0');

const contentRef = ref<HTMLElement | null>(null);
const contentRefFallback = ref<HTMLElement | null>(null);
const sectionRefs = ref<Record<string, HTMLElement>>({});
let observer: IntersectionObserver | null = null;
let isManualScrolling = false;

const setSectionRef = (el: unknown, key: string) => {
  if (el) sectionRefs.value[key] = el as HTMLElement;
};

const languages = [
  { value: 'ar', label: 'Arabic' },
  { value: 'zh', label: 'Chinese Simplified (简体中文)' },
  { value: 'zh-TW', label: 'Chinese Traditional (繁體中文)' },
  { value: 'en', label: 'English' },
  { value: 'fr', label: 'French (Français)' },
  { value: 'de', label: 'German (Deutsch)' },
  { value: 'it', label: 'Italian (Italiano)' },
  { value: 'ja', label: 'Japanese (日本語)' },
  { value: 'ko', label: 'Korean (한국어)' },
  { value: 'ms', label: 'Malay (Bahasa Melayu)' },
  { value: 'ru', label: 'Russian (Русский)' },
  { value: 'es', label: 'Spanish (Español)' },
];

const menuItems = computed(() => [
  { key: 'appearance', label: t('settings.appearance'), icon: Palette },
  { key: 'language', label: t('settings.language'), icon: Languages },
  { key: 'terminal', label: t('settings.terminal'), icon: Terminal },
  { key: 'shortcuts', label: t('settings.shortcuts'), icon: Keyboard },
  {
    key: 'terminalShortcuts',
    label: t('settings.terminalShortcuts'),
    icon: Zap,
  },
  { key: 'about', label: t('settings.about'), icon: Info },
]);

const isMac = isMacOSBrowser();
const cmdKey = isMac ? '⌘' : 'Ctrl';
const shiftKey = isMac ? '⇧' : 'Shift';

const shortcutList = computed(() => [
  { label: t('settings.newTab'), value: `${cmdKey}+T` },
  { label: t('settings.newLocalTab'), value: `${cmdKey}+${shiftKey}+T` },
  { label: t('settings.closeTab'), value: `${cmdKey}+W` },
  { label: t('settings.focusSearch'), value: `${cmdKey}+P` },
  { label: t('settings.openSettings'), value: `${cmdKey}+,` },
  { label: t('settings.quitApp'), value: `${cmdKey}+Q` },
  { label: t('settings.closeDialog'), value: 'Esc' },
]);

const terminalShortcutList = computed(() => [
  { label: t('settings.copy'), value: `${cmdKey}+C` },
  { label: t('settings.paste'), value: `${cmdKey}+V` },
  { label: t('settings.selectAll'), value: `${cmdKey}+A` },
  { label: t('settings.search'), value: `${cmdKey}+F` },
  { label: t('settings.clearTerminal'), value: `${cmdKey}+K` },
]);

const handleClose = () => {
  emit('update:visible', false);
};

const handleMenuClick = (key: string) => {
  isManualScrolling = true;
  activeMenu.value = key;
  const target = sectionRefs.value[key];
  if (target) {
    target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    // Reset manual scroll flag after animation
    setTimeout(() => {
      isManualScrolling = false;
    }, 1000);
  }
};

const initObserver = () => {
  if (observer) observer.disconnect();

  const options = {
    root: props.useTeleport ? contentRef.value : contentRefFallback.value,
    threshold: 0.2,
    rootMargin: '-20px 0px -70% 0px',
  };

  observer = new IntersectionObserver(entries => {
    if (isManualScrolling) return;

    entries.forEach(entry => {
      if (entry.isIntersecting) {
        // Find which section this element belongs to
        for (const [key, el] of Object.entries(sectionRefs.value)) {
          if (el === entry.target) {
            activeMenu.value = key;
            break;
          }
        }
      }
    });
  }, options);

  Object.values(sectionRefs.value).forEach(section => {
    observer?.observe(section);
  });
};

watch(
  () => props.visible,
  async visible => {
    if (visible) {
      isManualScrolling = true;
      activeMenu.value = props.initialSection || 'appearance';
      await nextTick();

      const target = sectionRefs.value[activeMenu.value];
      if (target) {
        target.scrollIntoView({ behavior: 'auto', block: 'start' });
      } else {
        // Reset scroll position to top when opening if no target
        const content = props.useTeleport
          ? contentRef.value
          : contentRefFallback.value;
        if (content) {
          content.scrollTop = 0;
        }
      }

      initObserver();
      // Short delay to allow initial intersection events to pass
      setTimeout(() => {
        isManualScrolling = false;
      }, 150);
    }
  }
);

const handleLanguageChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  const lang = target.value;
  locale.value = lang;
  localStorage.setItem('language', lang);
};

const handleCursorStyleChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  settingsStore.setCursorStyle(target.value as CursorStyle);
};

const handleThemeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  const theme = target.value as ThemeMode;
  selectedTheme.value = theme;
  themeManager.setTheme(theme);
};

onMounted(async () => {
  selectedTheme.value = themeManager.getTheme();
  if (props.visible) {
    initObserver();
  }

  try {
    appVersion.value = await getVersion();
  } catch (err) {
    console.warn('Failed to get app version:', err);
  }
});

onUnmounted(() => {
  if (observer) observer.disconnect();
});
</script>

<style scoped>
.settings-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(2px);
  border-radius: inherit;
  overflow: hidden;
}

.settings-panel {
  width: 760px;
  height: 540px;
  /* Base styles provided by .modal-system-panel */
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
  /* Explicitly apply top rounded corners and use negative margins to eliminate border gaps */
  margin: -1px -1px 0 -1px;
  border-top-left-radius: var(--radius-2xl);
  border-top-right-radius: var(--radius-2xl);
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
  width: 170px;
  background-color: var(--color-bg-tertiary);
  padding: 12px 8px;
  flex-shrink: 0;
  /* Explicitly apply bottom-left rounded corners to align with the parent container */
  border-bottom-left-radius: var(--radius-2xl);
}

.settings-menu {
  display: flex;
  flex-direction: column;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: none;
  background-color: transparent;
  text-align: left;
  cursor: pointer;
  font-size: 12px;
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
  width: 14px;
  height: 14px;
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
  padding: 20px 24px;
  overflow-y: auto;
  background-color: var(--color-bg-primary);
  /* Explicitly apply bottom-right rounded corners to align with the parent container */
  border-bottom-right-radius: var(--radius-2xl);
  scroll-behavior: smooth;
}

.content-section {
  max-width: 520px;
  margin-bottom: 32px;
}

.content-section:last-child {
  margin-bottom: 300px;
}

.section-title {
  font-size: 11px;
  font-weight: 700;
  color: var(--color-text-tertiary);
  margin: 0 0 8px 0;
  padding-bottom: 0;
  border-bottom: none;
  text-transform: uppercase;
  letter-spacing: 0.8px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background-color: var(--color-bg-elevated);
  border: 0.5px solid var(--color-border-tertiary);
  border-radius: var(--radius-md);
  margin-bottom: 4px;
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
  font-size: 12px;
  color: var(--color-text-primary);
  font-weight: 400;
}

/* Remove custom input styles - use common modal styles instead */
.setting-input,
.setting-select {
  /* Apply common modal input styles */
  width: 140px;
}

.setting-input:focus,
.setting-select:focus {
  /* Use common modal input focus styles */
}

.setting-checkbox {
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: #4a90e2;
}

.shortcut-key {
  padding: 2px 6px;
  background-color: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-secondary);
}

.about-info {
  padding: 8px 0;
}

.about-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 20px;
}

.about-logo {
  width: 48px;
  height: 48px;
  background: white;
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-md);
  overflow: hidden;
}

.logo-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.about-title-group {
  display: flex;
  flex-direction: column;
}

.about-app-name {
  font-size: 18px;
  font-weight: 700;
  margin: 0;
  color: var(--color-text-primary);
}

.about-version {
  font-size: 12px;
  color: var(--color-text-tertiary);
  margin: 2px 0 0 0;
}

.about-content {
  background-color: var(--color-bg-secondary);
  padding: 16px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border-secondary);
}

.about-desc {
  margin: 0 0 16px 0;
  font-size: 13px;
  color: var(--color-text-secondary);
  line-height: 1.6;
}

.about-meta {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 16px;
  border-top: 1px solid var(--color-border-tertiary);
}

.meta-item {
  display: flex;
  align-items: center;
  font-size: 12px;
}

.meta-label {
  width: 100px;
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.meta-value {
  color: var(--color-text-primary);
  font-weight: 500;
}

.meta-link {
  color: var(--color-primary);
  text-decoration: none;
  font-weight: 500;
  transition: opacity 0.2s;
}

.meta-link:hover {
  text-decoration: underline;
  opacity: 0.8;
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
