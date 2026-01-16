<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { themeManager } from '@/core/utils/theme-manager';

const emit = defineEmits(['complete']);

const { locale, availableLocales } = useI18n({ useScope: 'global' });

const showContent = ref(false);
const showOptions = ref(false);
const selectedLanguage = ref(locale.value);
const selectedTheme = ref(themeManager.getTheme());

const logoSrc = computed(() => {
  return '/welcome-image.png';
});

// Based on theme, we'll apply different styles via CSS
const themeClass = computed(() => {
  return selectedTheme.value === 'auto'
    ? `theme-${themeManager.getActualTheme()}`
    : `theme-${selectedTheme.value}`;
});

// Map of language codes to display names
const languageNames: Record<string, string> = {
  en: 'English',
  zh: '简体中文',
  'zh-TW': '繁體中文',
  ja: '日本語',
  ko: '한국어',
  fr: 'Français',
  de: 'Deutsch',
  ru: 'Русский',
  es: 'Español',
  ms: 'Bahasa Melayu',
  it: 'Italiano',
  ar: 'العربية',
};

const detectLanguage = async () => {
  // Simulate async detection
  await new Promise(resolve => setTimeout(resolve, 500));

  const browserLang = navigator.language;
  console.log('Detected browser language:', browserLang);

  if (browserLang.startsWith('zh')) {
    if (browserLang.includes('TW') || browserLang.includes('HK')) {
      return 'zh-TW';
    }
    return 'zh';
  }

  const langPrefix = browserLang.split('-')[0];
  if (availableLocales.indexOf(langPrefix) !== -1) {
    return langPrefix;
  }

  return 'en';
};

onMounted(async () => {
  showContent.value = true;

  // 1. Initial logo centered (handled by CSS)

  // 2. Async language detection
  const detected = await detectLanguage();
  selectedLanguage.value = detected;

  // 3. After 2.2 seconds (Golden ratio for breathing animation peak),
  // animate logo to left and show options
  setTimeout(() => {
    showOptions.value = true;
  }, 2200);
});

const handleThemeSelect = (theme: 'auto' | 'light' | 'dark') => {
  selectedTheme.value = theme;
  themeManager.setTheme(theme);
};

const handleSave = () => {
  locale.value = selectedLanguage.value;
  localStorage.setItem('language', selectedLanguage.value);
  themeManager.setTheme(selectedTheme.value);
  localStorage.setItem('hasLaunched', 'true');
  emit('complete');
};
</script>

<template>
  <Transition name="fade">
    <div v-if="showContent" class="welcome-screen" :class="themeClass">
      <div class="welcome-container" :class="{ 'show-options': showOptions }">
        <!-- Logo Section -->
        <div class="logo-section">
          <div class="logo-icon">
            <img :src="logoSrc" alt="NexaShell Logo" class="logo-img" />
          </div>
          <h1 class="app-name">NexaShell</h1>
        </div>

        <!-- Language and Theme Selector Section -->
        <div
          class="options-section"
          :style="{
            opacity: showOptions ? 1 : 0,
            pointerEvents: showOptions ? 'auto' : 'none',
          }"
        >
          <div class="options-content">
            <h2>{{ $t('welcome.title') }}</h2>
            <p class="subtitle">
              {{ $t('welcome.subtitle') }}
            </p>

            <div class="selection-group">
              <h3>{{ $t('welcome.language') }}</h3>
              <div class="selection-grid">
                <button
                  v-for="lang in availableLocales"
                  :key="lang"
                  class="select-btn"
                  :class="{ active: selectedLanguage === lang }"
                  @click="selectedLanguage = lang"
                >
                  {{ languageNames[lang] || lang }}
                </button>
              </div>
            </div>

            <div class="selection-group">
              <h3>{{ $t('welcome.appearance') }}</h3>
              <div class="selection-grid theme-grid">
                <button
                  class="select-btn"
                  :class="{ active: selectedTheme === 'light' }"
                  @click="handleThemeSelect('light')"
                >
                  {{ $t('welcome.themeLight') }}
                </button>
                <button
                  class="select-btn"
                  :class="{ active: selectedTheme === 'dark' }"
                  @click="handleThemeSelect('dark')"
                >
                  {{ $t('welcome.themeDark') }}
                </button>
                <button
                  class="select-btn"
                  :class="{ active: selectedTheme === 'auto' }"
                  @click="handleThemeSelect('auto')"
                >
                  {{ $t('welcome.themeSystem') }}
                </button>
              </div>
            </div>

            <button class="save-btn" @click="handleSave">
              {{ $t('welcome.getStarted') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.welcome-screen {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background-color: var(--color-bg-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-primary);
  overflow: hidden;
}

.welcome-container {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  max-width: 1200px;
  height: 100%;
  position: relative;
  /* No padding transition to avoid layout thrash */
}

.logo-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  transition: transform 1.2s cubic-bezier(0.6, 0, 0.4, 1);
  flex-shrink: 0;
  position: absolute;
  left: 50%;
  top: 50%;
  /* Use translate(-50%, -50%) for perfect centering, combined with scale */
  transform: translate(-50%, -50%) scale(2.2);
  will-change: transform;
  z-index: 10;
}

.welcome-container.show-options .logo-section {
  /* Move from center to the left side: -50% (center) - 250px (offset) */
  transform: translate(calc(-50% - 240px), -50%) scale(0.8);
}

.logo-icon {
  width: 180px;
  height: 180px;
  margin-bottom: 20px;
  filter: drop-shadow(0 0 40px rgba(59, 130, 246, 0.4));
  animation: breathing 4s infinite ease-in-out;
  /* Hardware acceleration */
  transform: translateZ(0);
  backface-visibility: hidden;
  will-change: transform, filter, opacity;
  transition: filter 0.5s ease;
}

.theme-dark .logo-img {
  /* This will intelligently invert the colors for dark mode */
  filter: invert(0.9) hue-rotate(180deg) brightness(1.2);
}

.logo-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.app-name {
  font-size: 3rem;
  font-weight: 800;
  background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  margin: 0;
  opacity: 0.9;
}

.options-section {
  position: absolute;
  left: 50%;
  top: 50%;
  /* Positioned to the right of the center */
  transform: translate(calc(-50% + 180px), -50%);
  width: 500px;
  transition:
    transform 1.2s cubic-bezier(0.6, 0, 0.4, 1),
    opacity 1.2s cubic-bezier(0.6, 0, 0.4, 1);
  will-change: transform, opacity;
}

.welcome-container.show-options .options-section {
  transform: translate(calc(-50% + 200px), -50%);
}

.options-content {
  max-width: 480px;
}

h2 {
  font-size: 2.5rem;
  margin-bottom: 8px;
  font-weight: 700;
}

.subtitle {
  font-size: 1.1rem;
  color: var(--color-text-secondary);
  line-height: 1.6;
  margin-bottom: 32px;
  opacity: 0.8;
}

.selection-group {
  margin-bottom: 24px;
}

h3 {
  font-size: 0.9rem;
  text-transform: uppercase;
  color: var(--color-text-secondary);
  margin-bottom: 12px;
  letter-spacing: 0.05em;
}

.selection-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 10px;
}

.select-btn {
  padding: 10px 14px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  font-size: 0.95rem;
  cursor: pointer;
  transition: all 0.2s;
  text-align: center;
}

.select-btn:hover {
  border-color: var(--color-primary);
}

.select-btn.active {
  background: var(--color-interactive-selected);
  border-color: var(--color-primary);
  color: var(--color-primary);
  font-weight: 500;
}

.save-btn {
  width: 100%;
  margin-top: 16px;
  padding: 16px;
  background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%);
  border: none;
  border-radius: var(--radius-lg);
  color: white;
  font-weight: 600;
  font-size: 1.1rem;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 4px 20px rgba(59, 130, 246, 0.3);
}

.save-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(59, 130, 246, 0.4);
}

.save-btn:active {
  transform: translateY(0);
}

@keyframes breathing {
  0% {
    transform: translateZ(0) scale(1);
    filter: drop-shadow(0 0 30px rgba(59, 130, 246, 0.3));
    opacity: 0.95;
  }
  50% {
    transform: translateZ(0) scale(1.04);
    filter: drop-shadow(0 0 60px rgba(59, 130, 246, 0.5));
    opacity: 1;
  }
  100% {
    transform: translateZ(0) scale(1);
    filter: drop-shadow(0 0 30px rgba(59, 130, 246, 0.3));
    opacity: 0.95;
  }
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.8s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
