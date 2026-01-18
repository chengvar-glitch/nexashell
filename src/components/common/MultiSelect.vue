<template>
  <div class="multi-select">
    <label v-if="label" :for="`${itemType}-select`" class="select-label">
      {{ label }}
    </label>
    <div class="select-container">
      <!-- Input with dropdown trigger -->
      <div class="input-wrapper">
        <div class="selected-items">
          <span
            v-for="itemId in selectedItems"
            :key="itemId"
            class="selected-item"
          >
            <span class="tag-text">{{ getItemName(itemId) }}</span>
            <button
              type="button"
              class="remove-btn"
              :aria-label="`Remove ${getItemName(itemId)}`"
              @click="removeItem(itemId)"
            >
              ✕
            </button>
          </span>
        </div>
        <input
          :id="`${itemType}-select`"
          v-model="searchQuery"
          type="text"
          :placeholder="selectedItems.length === 0 ? placeholder : ''"
          class="select-input"
          @focus="isOpen = true"
          @input="isOpen = true"
          @blur="handleInputBlur"
          @keydown.escape="isOpen = false"
          @keydown.enter="addNewItem"
          @keydown.tab="isOpen = false"
        />
      </div>

      <!-- Dropdown menu -->
      <transition name="dropdown">
        <div 
          v-if="isOpen" 
          class="dropdown-menu"
          :class="{ 'dropdown-menu-up': dropdownDirection === 'up' }"
          :style="{ maxHeight: dropdownMaxHeight }"
        >
          <!-- Existing items -->
          <div v-if="filteredItems.length > 0" class="item-list">
            <div
              v-for="item in filteredItems"
              :key="item.id"
              class="item"
              @click="toggleItem(item.id)"
            >
              <input
                type="checkbox"
                :checked="selectedItems.includes(item.id)"
                class="item-checkbox"
              />
              <span class="item-name">{{ item.name }}</span>
            </div>
          </div>

          <!-- Divider if there are both existing and new item option -->
          <div v-if="filteredItems.length > 0 && searchQuery.trim()" class="divider" />

          <!-- New item input -->
          <div v-if="searchQuery.trim()" class="new-item-section">
            <button
              type="button"
              class="new-item-btn"
              @click="addNewItem"
            >
              <span class="plus-icon">+</span>
              {{ createItemText }}: "{{ searchQuery }}"
            </button>
          </div>

          <!-- Empty state -->
          <div v-if="filteredItems.length === 0 && !searchQuery.trim()" class="empty-state">
            {{ emptyText }}
          </div>
        </div>
      </transition>
    </div>
  </div>
</template>

<script setup lang="ts" generic="T extends { id: string; name: string; sort: number; created_at: string; updated_at: string }">
import { ref, computed, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';

interface Props {
  modelValue?: string[];
  items?: T[];
  label?: string;
  placeholder?: string;
  createItemText?: string;
  emptyText?: string;
  allowCreate?: boolean;
  itemType: 'tag' | 'group'; // 'tag' or 'group' to determine event and command
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: () => [],
  items: () => [],
  label: '',
  placeholder: 'Select items...',
  createItemText: 'Create',
  emptyText: 'No items available',
  allowCreate: true,
});

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  'item-added': [item: any];
}>();

const isOpen = ref(false);
const searchQuery = ref('');
const selectedItems = ref<string[]>(props.modelValue || []);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const allItems = ref<any[]>(props.items || []);
const dropdownDirection = ref<'down' | 'up'>('up');

// Watch for changes to the items prop and update local state
watch(() => props.items, (newItems) => {
  allItems.value = newItems || [];
}, { immediate: true, deep: true });

// Computed: filter items based on search query
const filteredItems = computed(() => {
  const query = searchQuery.value.toLowerCase().trim();
  if (!query) {
    return allItems.value.filter(item => !selectedItems.value.includes(item.id));
  }
  return allItems.value.filter(
    item =>
      item.name.toLowerCase().includes(query) &&
      !selectedItems.value.includes(item.id)
  );
});

// Computed: calculate dropdown max-height based on number of visible items
const dropdownMaxHeight = computed(() => {
  // Each item is approximately 24px
  const itemHeight = 24;
  const maxVisibleItems = 5;
  const itemListPadding = 12; // top + bottom padding of item-list (6px + 6px)
  const dividerHeight = 5; // 1px height + 4px margin
  const newItemSectionHeight = 30; // button with padding
  const totalMaxHeight = itemHeight * maxVisibleItems + itemListPadding + dividerHeight + newItemSectionHeight;
  
  // Always return a pixel value to ensure proper scrolling
  // If actual items count is less than or equal to maxVisibleItems, use auto-height but as pixel value
  if (filteredItems.value.length <= maxVisibleItems) {
    const autoHeight = itemHeight * filteredItems.value.length + itemListPadding;
    return `${autoHeight}px`;
  }
  
  // Otherwise, set max-height and enable scrolling
  return `${totalMaxHeight}px`;
});

const toggleItem = (itemId: string) => {
  if (selectedItems.value.includes(itemId)) {
    removeItem(itemId);
  } else {
    selectedItems.value.push(itemId);
    emit('update:modelValue', selectedItems.value);
    searchQuery.value = '';
  }
};

const removeItem = (itemId: string) => {
  selectedItems.value = selectedItems.value.filter(id => id !== itemId);
  emit('update:modelValue', selectedItems.value);
};

const getItemName = (itemId: string): string => {
  return allItems.value.find(item => item.id === itemId)?.name || itemId;
};

const addNewItem = async () => {
  const name = searchQuery.value.trim();
  if (!name || !props.allowCreate) {
    return;
  }

  // Check if item with same name already exists
  const existingItem = allItems.value.find(
    item => item.name.toLowerCase() === name.toLowerCase()
  );

  if (existingItem) {
    // Item already exists, just select it
    if (!selectedItems.value.includes(existingItem.id)) {
      selectedItems.value.push(existingItem.id);
      emit('update:modelValue', selectedItems.value);
    }
    searchQuery.value = '';
    await nextTick();
    isOpen.value = false;
    return;
  }

  try {
    // Determine command name based on itemType
    const commandName = props.itemType === 'tag' ? 'add_tag' : 'add_group';
    const eventName = props.itemType === 'tag' ? APP_EVENTS.TAGS_UPDATED : APP_EVENTS.GROUPS_UPDATED;

    // Call Tauri command to create item
    const newItemId = await invoke<string>(commandName, { name });

    // Create the new item object
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const newItem: any = {
      id: newItemId,
      name,
      sort: 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    // Add to local items list
    allItems.value.push(newItem);

    // Select the newly created item
    selectedItems.value.push(newItemId);
    emit('update:modelValue', selectedItems.value);
    emit('item-added', newItem);

    // Emit event to notify other components (e.g., home page) to refresh
    eventBus.emit(eventName);

    // Clear search and close dropdown
    searchQuery.value = '';
    await nextTick();
    isOpen.value = false;
  } catch (error) {
    console.error(`Failed to create ${props.itemType}:`, error);
  }
};

const handleInputBlur = async () => {
  // Delay closing to allow click events on dropdown items to register
  await nextTick();
  setTimeout(() => {
    if (!isOpen.value) return;
    isOpen.value = false;
  }, 100);
};

const checkDropdownPosition = async () => {
  await nextTick();
  const container = document.querySelector('.select-container') as HTMLElement;
  const menu = document.querySelector('.dropdown-menu') as HTMLElement;
  
  if (!container || !menu) return;
  
  const containerRect = container.getBoundingClientRect();
  const menuHeight = menu.offsetHeight;
  const spaceBelow = window.innerHeight - containerRect.bottom;
  const spaceAbove = containerRect.top;
  
  // Default to opening upwards
  // Only switch to downwards if there's not enough space above and more space below
  if (spaceAbove < menuHeight + 10 && spaceBelow > menuHeight + 10) {
    dropdownDirection.value = 'down';
  } else {
    dropdownDirection.value = 'up';
  }
};

watch(isOpen, async (newVal) => {
  if (newVal) {
    await checkDropdownPosition();
  }
});
</script>

<style scoped>
/* ==================== Layout ==================== */
.multi-select {
  width: 100%;
}

/* ==================== Label ==================== */
.select-label {
  display: block;
  margin-bottom: 4px;
  color: var(--color-text-primary);
  font-size: 0.9em;
  font-weight: 500;
  letter-spacing: -0.2px;
}

/* ==================== Input Wrapper ==================== */
.select-container {
  position: relative;
  width: 100%;
}

.input-wrapper {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  width: 100%;
  min-height: 34px;
  padding: 6px 8px;
  background: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  cursor: text;
  box-sizing: border-box;
  transition: 
    border-color var(--transition-fast),
    background-color var(--transition-fast),
    box-shadow var(--transition-fast);
}

.input-wrapper:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px rgba(var(--color-primary-rgb), 0.08);
}

/* ==================== Selected Items (Tags) ==================== */
.selected-items {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.selected-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border-radius: 6px;
  font-size: 0.8em;
  line-height: 1;
  border: 1px solid var(--color-border-secondary);
  transition: 
    background-color var(--transition-fast),
    border-color var(--transition-fast);
}

.selected-item:hover {
  background-color: var(--color-bg-tertiary);
  border-color: var(--color-border-primary);
}

.tag-text {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ==================== Remove Button ==================== */
.remove-btn {
  background: none;
  border: none;
  color: var(--color-text-secondary);
  cursor: pointer;
  padding: 0 2px;
  font-size: 0.9em;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  transition: 
    color var(--transition-fast),
    transform var(--transition-fast);
  border-radius: 3px;
}

.remove-btn:hover {
  color: var(--color-text-primary);
  transform: scale(1.1);
}

.remove-btn:active {
  transform: scale(0.95);
}

/* ==================== Input Field ==================== */
.select-input {
  flex: 1;
  min-width: 60px;
  border: none;
  background: transparent;
  padding: 4px 4px;
  font-size: 0.85em;
  color: var(--color-text-primary);
  outline: none;
  transition: background-color var(--transition-fast);
}

.select-input::placeholder {
  color: var(--color-text-secondary);
}

.select-input:focus {
  background-color: transparent;
}

/* ==================== Dropdown Menu ==================== */
.dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 8px;
  background: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
  z-index: 10000;
  overflow-y: auto;
  overflow-x: hidden;
  animation: slideDown var(--transition-fast) ease;
  min-width: 100%;
  scrollbar-width: thin;
  scrollbar-color: var(--color-border-primary) transparent;
}

/* Webkit scrollbar styling for Chrome/Safari */
.dropdown-menu::-webkit-scrollbar {
  width: 6px;
}

.dropdown-menu::-webkit-scrollbar-track {
  background-color: transparent;
}

.dropdown-menu::-webkit-scrollbar-thumb {
  background-color: var(--color-border-primary);
  border-radius: 3px;
}

.dropdown-menu::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-text-tertiary);
}

.dropdown-menu-up {
  top: auto;
  bottom: 100%;
  margin-top: 0;
  margin-bottom: 8px;
  animation: slideUp var(--transition-fast) ease;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* ==================== Item List ==================== */
.item-list {
  padding: 6px 0;
}

.item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  cursor: pointer;
  font-size: 0.85em;
  transition: 
    background-color var(--transition-fast),
    color var(--transition-fast);
}

.item:hover {
  background-color: var(--color-bg-secondary);
}

.item:active {
  background-color: var(--color-bg-tertiary);
}

.item-checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: var(--color-primary);
  transition: transform var(--transition-fast);
}

.item-checkbox:hover {
  transform: scale(1.05);
}

.item-name {
  flex: 1;
  color: var(--color-text-primary);
  font-size: 0.85em;
}

/* ==================== Divider ==================== */
.divider {
  height: 1px;
  background: var(--color-border-secondary);
  margin: 4px 0;
}

/* ==================== New Item Section ==================== */
.new-item-section {
  padding: 6px 0;
}

.new-item-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--color-primary);
  font-size: 0.85em;
  text-align: left;
  transition: 
    background-color var(--transition-fast),
    color var(--transition-fast);
}

.new-item-btn:hover {
  background-color: var(--color-bg-secondary);
  color: var(--color-primary);
}

.new-item-btn:active {
  background-color: var(--color-bg-tertiary);
}

.plus-icon {
  font-weight: 600;
  font-size: 1.1em;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
}

/* ==================== Empty State ==================== */
.empty-state {
  padding: 8px 10px;
  text-align: center;
  color: var(--color-text-secondary);
  font-size: 0.85em;
}

/* ==================== Transitions ==================== */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all var(--transition-fast) ease;
}

.dropdown-enter-from {
  opacity: 0;
  transform: translateY(-4px);
}

.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
