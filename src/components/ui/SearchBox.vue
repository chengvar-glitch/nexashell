<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { isMacOS, isWindows } from '../../utils/tauri-system';

interface Props {
  placeholder?: string;
  modelValue?: string;
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: 'Quick search (Cmd+K)',
  modelValue: ''
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
  'search': [value: string];
  'focus': [];
  'blur': [];
  'keydown': [event: KeyboardEvent];
  'keyup': [event: KeyboardEvent];
}>();

const searchInputRef = ref<HTMLInputElement | null>(null);
const isMacOS_OS = ref(false);
const isWindowsOS = ref(false);
const isFocused = ref(false);

const onInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value);
};

const onSearch = () => {
  emit('search', props.modelValue);
};

defineExpose({
  focus: () => {
    if (searchInputRef.value) {
      searchInputRef.value.focus();
    }
  }
});

const handleFocus = () => {
  isFocused.value = true;
  if (searchInputRef.value) {
    searchInputRef.value.select();
  }
  emit('focus');
};

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
    const [isMac, isWin] = await Promise.all([isMacOS(), isWindows()]);
    isMacOS_OS.value = isMac;
    isWindowsOS.value = isWin;
  } catch (error) {
    console.error('Failed to detect platform:', error);
    isMacOS_OS.value = navigator.userAgent.includes('Mac');
    isWindowsOS.value = navigator.userAgent.includes('Windows');
  }
});

const shortcutText = computed(() => {
  if (isMacOS_OS.value) {
    return 'Cmd+K';
  } else if (isWindowsOS.value) {
    return 'Ctrl+K';
  } else {
    return 'Ctrl+K';
  }
});

const dynamicPlaceholder = computed(() => {
  return `Quick search (${shortcutText.value})`;
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
      @keypress.enter="onSearch"
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
  max-width: 520px;
  min-width: 300px;
  padding: 9px 18px;
  padding-left: 40px;
  border-radius: var(--radius-2xl);
  border: 1px solid var(--color-border-secondary);
  background-color: var(--color-bg-elevated);
  font-size: 13px;
  color: var(--color-text-primary);
  font-family: inherit;
  outline: none;
  transition: all var(--transition-base);
  box-shadow: var(--shadow-sm);
  backdrop-filter: var(--blur-light);
  text-align: left;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'%3E%3C/circle%3E%3Cpath d='m21 21-4.3-4.3'%3E%3C/path%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: 14px center;
  background-size: 16px 16px;
}

.search-input::placeholder {
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
  border-radius: var(--radius-2xl) var(--radius-2xl) 0 0;
  border-bottom-color: transparent;
}
</style>