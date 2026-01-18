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
            <span>{{ getItemName(itemId) }}</span>
            <button
              type="button"
              class="remove-btn"
              :aria-label="`Remove ${getItemName(itemId)}`"
              @click="removeItem(itemId)"
            >
              ×
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
const dropdownDirection = ref<'down' | 'up'>('down');

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
  // Each item is approximately 24px (6px padding + 12px text + 6px padding)
  const itemHeight = 24;
  const maxVisibleItems = 6;
  const padding = 8; // padding for item-list and other elements
  const maxHeight = itemHeight * maxVisibleItems + padding;
  
  // If actual items count is less than or equal to maxVisibleItems, show all without scrolling
  if (filteredItems.value.length <= maxVisibleItems) {
    return 'auto';
  }
  
  // Otherwise, set max-height and enable scrolling
  return `${maxHeight}px`;
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
  
  // If there's not enough space below (less than menu height + 10px buffer)
  // and there's more space above, open menu upwards
  if (spaceBelow < menuHeight + 10 && spaceAbove > menuHeight + 10) {
    dropdownDirection.value = 'up';
  } else {
    dropdownDirection.value = 'down';
  }
};

watch(isOpen, async (newVal) => {
  if (newVal) {
    await checkDropdownPosition();
  }
});
</script>

<style scoped>
.multi-select {
  width: 100%;
}

.select-label {
  display: block;
  margin-bottom: 2px;
  color: var(--color-text-primary);
  font-size: 0.85em;
  font-weight: 500;
}

.select-container {
  position: relative;
  width: 100%;
}

.input-wrapper {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  align-items: center;
  min-height: 28px;
  padding: 3px 6px;
  background: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: 0px;
  cursor: text;
  transition: border-color 0.15s ease;
}

.input-wrapper:focus-within {
  border-color: var(--color-primary);
  box-shadow: none;
}

.selected-items {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
}

.selected-item {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 2px 6px;
  background: var(--color-primary);
  color: white;
  border-radius: 2px;
  font-size: 0.8em;
  line-height: 1;
}

.remove-btn {
  background: none;
  border: none;
  color: white;
  cursor: pointer;
  padding: 0;
  font-size: 1em;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  transition: none;
}

.remove-btn:hover {
  opacity: 0.8;
}

.select-input {
  flex: 1;
  min-width: 80px;
  border: none;
  background: transparent;
  padding: 2px 3px;
  font-size: 0.85em;
  color: var(--color-text-primary);
  outline: none;
}

.select-input::placeholder {
  color: var(--color-text-tertiary);
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 2px;
  background: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: 0px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  z-index: 1000;
  overflow-y: auto;
  overflow-x: hidden;
}

.dropdown-menu-up {
  top: auto;
  bottom: 100%;
  margin-top: 0;
  margin-bottom: 2px;
}

.item-list {
  padding: 2px 0;
}

.item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  cursor: pointer;
  transition: none;
}

.item:hover {
  background-color: var(--color-bg-secondary);
}

.item-checkbox {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--color-primary);
}

.item-name {
  flex: 1;
  color: var(--color-text-primary);
  font-size: 0.85em;
}

.divider {
  height: 1px;
  background: var(--color-border-primary);
  margin: 2px 0;
}

.new-item-section {
  padding: 2px 0;
}

.new-item-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--color-primary);
  font-size: 0.85em;
  text-align: left;
  transition: none;
}

.new-item-btn:hover {
  background-color: var(--color-bg-secondary);
}

.plus-icon {
  font-weight: 600;
  font-size: 0.95em;
}

.empty-state {
  padding: 8px;
  text-align: center;
  color: var(--color-text-tertiary);
  font-size: 0.85em;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.1s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
}
</style>
