<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  isMacOSBrowser,
  isWindowsBrowser,
  formatShortcut,
} from '@/core/utils/platform/platform-detection';

/**
 * SearchBox Component
 *
 * A reusable search input component with platform-specific shortcut hints
 * and visual states for integration with complex search interfaces.
 */

interface Props {
  /** Placeholder text for the input */
  placeholder?: string;
  /** Reactive model value for search query */
  modelValue?: string;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
  /** Triggered when the user presses Enter */
  search: [value: string];
  focus: [];
  blur: [];
  input: [value: string];
  keydown: [event: KeyboardEvent];
  keyup: [event: KeyboardEvent];
}>();

const { t } = useI18n({ useScope: 'global' });
const searchInputRef = ref<HTMLInputElement | null>(null);
const isMacOS_OS = ref(false);
const isWindowsOS = ref(false);
const isFocused = ref(false);

/**
 * Updates the model value on every input event.
 */
const onInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value);
  emit('input', target.value);
};

/**
 * Emits search event when Enter key is pressed.
 */
const onSearch = () => {
  emit('search', props.modelValue);
};

// Expose methods to parent components
defineExpose({
  /** Provides programmatic focus to the input element */
  focus: () => {
    if (searchInputRef.value) {
      searchInputRef.value.focus();
    }
  },
});

/**
 * Handles input focus state and selects text for quick replacement.
 */
const handleFocus = () => {
  isFocused.value = true;
  if (searchInputRef.value) {
    searchInputRef.value.select();
  }
  emit('focus');
};

/**
 * Handles input blur state.
 */
const handleBlur = () => {
  isFocused.value = false;
  emit('blur');
};

const handleKeyDown = (event: KeyboardEvent) => {
  emit('keydown', event);
};

const handleKeyUp = (event: KeyboardEvent) => {
  emit('keyup', event);
};

onMounted(async () => {
  try {
    isMacOS_OS.value = await isMacOSBrowser();
    isWindowsOS.value = await isWindowsBrowser();
  } catch (error) {
    console.error('Failed to detect platform:', error);
    isMacOS_OS.value = false;
    isWindowsOS.value = false;
  }
});

/**
 * Computes platform-aware shortcut text for the placeholder.
 */
const shortcutText = computed(() => formatShortcut('Cmd+P'));

/**
 * Generates the final placeholder string including the shortcut key.
 */
const dynamicPlaceholder = computed(() => {
  if (props.placeholder) return props.placeholder;
  return t('search.placeholder', { shortcut: shortcutText.value });
});
</script>

<template>
  <div class="search-container no-drag flex-h-center">
    <input
      ref="searchInputRef"
      type="text"
      :placeholder="dynamicPlaceholder"
      :value="modelValue"
      :class="['search-input', { 'search-input-focused': isFocused }]"
      @input="onInput"
      @keydown.enter="onSearch"
      @focus="handleFocus"
      @blur="handleBlur"
      @keydown="handleKeyDown"
      @keyup="handleKeyUp"
    />
  </div>
</template>

<style scoped>
.search-container {
  position: relative;
  padding: 0 8px;
}

.search-input {
  width: 100%;
  max-width: 850px;
  min-width: 450px;
  height: 30px;
  padding: 0 16px 0 38px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border-secondary);
  background-color: var(--color-bg-elevated);
  font-size: 13px;
  color: var(--color-text-primary);
  font-family: inherit;
  outline: none;
  transition: all var(--transition-base);
  box-shadow: var(--shadow-sm);
  backdrop-filter: var(--blur-light);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'%3E%3C/circle%3E%3Cpath d='m21 21-4.3-4.3'%3E%3C/path%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: 15px center;
  background-size: 16px 16px;
  text-align: left;
}

.search-input::placeholder {
  text-align: left;
  color: var(--color-text-placeholder);
  font-weight: 400;
}

.search-input:hover {
  border-color: var(--color-border-secondary);
  background-color: var(--color-bg-elevated);
  box-shadow: var(--shadow-md);
}

.search-input:focus {
  border-color: var(--color-primary);
  background-color: var(--color-bg-elevated);
  box-shadow: var(--focus-ring), var(--shadow-md);
}

@media (prefers-color-scheme: dark) {
  :root:not(.theme-light) .search-input:focus {
    box-shadow: var(--focus-ring-intense), var(--shadow-lg);
  }
}

:root.theme-dark .search-input:focus {
  box-shadow: var(--focus-ring-intense), var(--shadow-lg);
}

/* Focus state: rounded bottom corners removed to integrate with dropdown */
.search-input-focused {
  border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  border-bottom-color: transparent;
}
</style>
