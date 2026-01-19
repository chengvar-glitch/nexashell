<template>
  <div class="multi-select">
    <label v-if="label" :for="`${itemType}-select`" class="select-label">
      {{ label }}
    </label>
    <div class="select-container">
      <!-- Input with dropdown trigger -->
      <div ref="inputWrapperRef" class="input-wrapper" @click="isOpen = true">
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
              @click.stop="removeItem(itemId)"
            >
              <svg
                width="10"
                height="10"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="3"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </span>
        </div>
        <input
          :id="`${itemType}-select`"
          v-model="searchQuery"
          type="text"
          :placeholder="selectedItems.length === 0 ? placeholder : ''"
          class="select-input"
          autocomplete="off"
          @focus="isOpen = true"
          @input="isOpen = true"
          @blur="handleInputBlur"
          @keydown="onKeyDown"
        />
        <div class="dropdown-icon" :class="{ 'is-open': isOpen }">
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="6 9 12 15 18 9"></polyline>
          </svg>
        </div>
      </div>

      <!-- Dropdown menu -->
      <transition name="dropdown">
        <div
          v-if="isOpen"
          class="dropdown-menu"
          :class="{ 'dropdown-menu-up': dropdownDirection === 'up' }"
          :style="{ maxHeight: dropdownMaxHeight }"
          @mousedown.prevent
        >
          <!-- Existing items -->
          <div v-if="filteredItems.length > 0" class="item-list">
            <div
              v-for="(item, index) in filteredItems"
              :key="item.id"
              class="item"
              :class="{ highlighted: index === highlightedIndex }"
              @click="toggleItem(item.id)"
              @mouseenter="highlightedIndex = index"
            >
              <span class="item-name">{{ item.name }}</span>
            </div>
          </div>

          <!-- Divider if there are both existing and new item option -->
          <div
            v-if="filteredItems.length > 0 && searchQuery.trim() && allowCreate"
            class="divider"
          />

          <!-- New item input -->
          <div
            v-if="searchQuery.trim() && allowCreate"
            class="new-item-section"
          >
            <button
              type="button"
              class="new-item-btn"
              :class="{
                highlighted:
                  highlightedIndex === navigationItems.length - 1 &&
                  navigationItems.length > filteredItems.length,
              }"
              @click="addNewItem"
              @mouseenter="highlightedIndex = navigationItems.length - 1"
            >
              <span class="plus-icon">+</span>
              {{ createItemText }}: "{{ searchQuery }}"
            </button>
          </div>

          <!-- Empty state -->
          <div
            v-if="filteredItems.length === 0 && !searchQuery.trim()"
            class="empty-state"
          >
            {{ emptyText }}
          </div>
          <div
            v-else-if="
              filteredItems.length === 0 && searchQuery.trim() && !allowCreate
            "
            class="empty-state"
          >
            No matches found
          </div>
        </div>
      </transition>
    </div>
  </div>
</template>

<script setup lang="ts" generic="T extends MetadataItem">
import { ref, computed, nextTick, watch } from 'vue';
import type { MetadataItem } from '@/core/types/common';

interface Props {
  modelValue?: string[];
  items?: T[];
  label?: string;
  placeholder?: string;
  createItemText?: string;
  emptyText?: string;
  allowCreate?: boolean;
  itemType?: string;
  /**
   * Optional async function to create a new item.
   * If provided, the component will wait for it and use the returned item.
   * If not provided, it will create a temporary item with id "new:{name}".
   */
  onCreateItem?: (name: string) => Promise<T>;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: () => [],
  items: () => [],
  label: '',
  placeholder: 'Select items...',
  createItemText: 'Create',
  emptyText: 'No items available',
  allowCreate: true,
  itemType: 'item',
});

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
  'item-added': [item: T];
}>();

const isOpen = ref(false);
const isCreating = ref(false);
const searchQuery = ref('');
const highlightedIndex = ref(-1);
const selectedItems = ref<string[]>(props.modelValue || []);
const allItems = ref<T[]>(props.items || []);
const dropdownDirection = ref<'down' | 'up'>('up');
const inputWrapperRef = ref<HTMLElement | null>(null);

// Watch for changes to the items prop and update local state
watch(
  () => props.items,
  newItems => {
    allItems.value = [...(newItems || [])];
  },
  { immediate: true, deep: true }
);

// Computed: filter items based on search query
const filteredItems = computed(() => {
  const query = searchQuery.value.toLowerCase().trim();
  if (!query) {
    return allItems.value.filter(
      item => !selectedItems.value.includes(item.id)
    );
  }
  return allItems.value.filter(
    item =>
      item.name.toLowerCase().includes(query) &&
      !selectedItems.value.includes(item.id)
  );
});

// Reset highlighted index when items or search query change
watch([filteredItems, searchQuery], () => {
  highlightedIndex.value = -1;
});

// Computed: combine filtered items and "create new" option for keyboard navigation
const navigationItems = computed(() => {
  const items = [...filteredItems.value] as any[];
  if (searchQuery.value.trim() && props.allowCreate) {
    items.push({
      id: 'create_new_item_action',
      name: searchQuery.value.trim(),
    });
  }
  return items;
});

const dropdownMaxHeight = computed(() => {
  // Each item is approximately 32px now with better padding
  const itemHeight = 32;
  const maxVisibleItems = 6;
  const itemListPadding = 12; // top + bottom padding of item-list (6px + 6px)
  const dividerHeight = 5; // 1px height + 4px margin
  const newItemSectionHeight = 32; // button with padding

  let totalHeight = filteredItems.value.length * itemHeight + itemListPadding;
  if (searchQuery.value.trim() && props.allowCreate) {
    totalHeight += dividerHeight + newItemSectionHeight;
  }

  const maxHeight = itemHeight * maxVisibleItems + itemListPadding;
  return `${Math.min(totalHeight, maxHeight)}px`;
});

const toggleItem = (itemId: string) => {
  if (selectedItems.value.includes(itemId)) {
    removeItem(itemId);
  } else {
    selectedItems.value.push(itemId);
    emit('update:modelValue', selectedItems.value);
    searchQuery.value = '';
    highlightedIndex.value = -1;
  }
};

const removeItem = (itemId: string) => {
  selectedItems.value = selectedItems.value.filter(id => id !== itemId);
  emit('update:modelValue', selectedItems.value);
};

const getItemName = (itemId: string): string => {
  const item = allItems.value.find(item => item.id === itemId);
  if (item) return item.name;
  if (itemId.startsWith('new:')) return itemId.substring(4);
  return itemId;
};

const onKeyDown = (e: KeyboardEvent) => {
  if (isCreating.value) return;

  if (e.key === 'ArrowDown') {
    e.preventDefault();
    if (!isOpen.value) {
      isOpen.value = true;
      return;
    }
    highlightedIndex.value =
      (highlightedIndex.value + 1) % navigationItems.value.length;
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    if (!isOpen.value) {
      isOpen.value = true;
      return;
    }
    highlightedIndex.value =
      (highlightedIndex.value - 1 + navigationItems.value.length) %
      navigationItems.value.length;
  } else if (e.key === 'Enter') {
    if (isOpen.value && highlightedIndex.value !== -1) {
      e.preventDefault();
      const item = navigationItems.value[highlightedIndex.value];
      if (item.id === 'create_new_item_action') {
        addNewItem();
      } else {
        toggleItem(item.id);
      }
    } else if (searchQuery.value.trim()) {
      e.preventDefault();
      addNewItem();
    }
  } else if (
    e.key === 'Backspace' &&
    searchQuery.value === '' &&
    selectedItems.value.length > 0
  ) {
    removeItem(selectedItems.value[selectedItems.value.length - 1]);
  } else if (e.key === 'Escape') {
    isOpen.value = false;
  }
};

const addNewItem = async () => {
  const name = searchQuery.value.trim();
  if (!name || !props.allowCreate || isCreating.value) {
    return;
  }

  // Check if item with same name already exists
  const existingItem = allItems.value.find(
    item => item.name.toLowerCase() === name.toLowerCase()
  );

  if (existingItem) {
    if (!selectedItems.value.includes(existingItem.id)) {
      selectedItems.value.push(existingItem.id);
      emit('update:modelValue', selectedItems.value);
    }
    searchQuery.value = '';
    isOpen.value = false;
    return;
  }

  if (props.onCreateItem) {
    try {
      isCreating.value = true;
      const newItem = await props.onCreateItem(name);

      // Add to local items list and select it
      (allItems.value as T[]).push(newItem);
      selectedItems.value.push(newItem.id);

      emit('update:modelValue', selectedItems.value);
      emit('item-added', newItem as T);

      searchQuery.value = '';
      isOpen.value = false;
    } catch (error) {
      console.error('Failed to create item:', error);
    } finally {
      isCreating.value = false;
    }
  } else {
    // Client-side only (temp items)
    const tempId = `new:${name}`;
    const newItem = {
      id: tempId,
      name,
      sort: 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    } as T;

    (allItems.value as unknown[]).push(newItem);
    selectedItems.value.push(tempId);

    emit('update:modelValue', selectedItems.value);
    emit('item-added', newItem as T);

    searchQuery.value = '';
    isOpen.value = false;
  }
};

const handleInputBlur = async () => {
  // Delay closing to allow click events on dropdown items to register
  await nextTick();
  setTimeout(() => {
    if (!isOpen.value) return;
    isOpen.value = false;
  }, 120);
};

const checkDropdownPosition = async () => {
  await nextTick();
  if (!inputWrapperRef.value) return;
  const menu = inputWrapperRef.value.nextElementSibling as HTMLElement;
  if (!menu) return;

  const containerRect = inputWrapperRef.value.getBoundingClientRect();
  const menuHeight = menu.offsetHeight;
  const spaceBelow = window.innerHeight - containerRect.bottom;
  const spaceAbove = containerRect.top;

  // Prefer opening downwards if there's space, otherwise upwards
  if (spaceBelow > menuHeight + 20 || spaceBelow > spaceAbove) {
    dropdownDirection.value = 'down';
  } else {
    dropdownDirection.value = 'up';
  }
};

watch(isOpen, async newVal => {
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
  gap: 4px;
  align-items: center;
  width: 100%;
  min-height: 34px;
  padding: 4px 8px;
  background: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  cursor: text;
  box-sizing: border-box;
  transition:
    border-color var(--transition-base),
    background-color var(--transition-base),
    box-shadow var(--transition-base);
}

.input-wrapper:hover {
  border-color: var(--color-border-hover);
}

.input-wrapper:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px rgba(var(--color-primary-rgb), 0.12);
}

/* ==================== Selected Items (Tags) ==================== */
.selected-items {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.selected-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border-radius: 4px;
  font-size: 0.75em;
  font-weight: 500;
  line-height: normal;
  border: 1px solid var(--color-border-secondary);
  transition: all var(--transition-fast);
  user-select: none;
}

.selected-item:hover {
  background-color: var(--color-bg-tertiary);
  border-color: var(--color-border-primary);
}

.tag-text {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ==================== Remove Button ==================== */
.remove-btn {
  background: transparent;
  border: none;
  color: var(--color-text-tertiary);
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  transition: all var(--transition-fast);
  border-radius: 50%;
}

.remove-btn:hover {
  color: #ff4757;
  background-color: rgba(255, 71, 87, 0.1);
}

/* ==================== Input Field ==================== */
.select-input {
  flex: 1;
  min-width: 40px;
  border: none;
  background: transparent;
  padding: 4px 2px;
  font-size: 0.85em;
  color: var(--color-text-primary);
  outline: none;
}

.select-input::placeholder {
  color: var(--color-text-placeholder);
}

/* ==================== Dropdown Icon ==================== */
.dropdown-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary);
  margin-left: auto;
  transition: transform var(--transition-base);
  pointer-events: none;
}

.dropdown-icon.is-open {
  transform: rotate(180deg);
  color: var(--color-primary);
}

/* ==================== Dropdown Menu ==================== */
.dropdown-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: var(--color-bg-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-xl);
  z-index: 1000;
  overflow-y: auto;
  scrollbar-width: none; /* Hide scrollbar for clean look */
}

.dropdown-menu::-webkit-scrollbar {
  display: none;
}

.dropdown-menu-up {
  top: auto;
  bottom: calc(100% + 4px);
}

/* ==================== Item List ==================== */
.item-list {
  padding: 4px;
}

.item {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  cursor: pointer;
  border-radius: 4px;
  transition: all var(--transition-fast);
}

.item:hover,
.item.highlighted {
  background-color: var(--color-bg-secondary);
}

.item-name {
  flex: 1;
  color: var(--color-text-primary);
  font-size: 0.85em;
}

.item.highlighted .item-name {
  color: var(--color-primary);
  font-weight: 500;
}

/* ==================== Divider ==================== */
.divider {
  height: 1px;
  background: var(--color-border-secondary);
  margin: 2px 4px;
}

/* ==================== New Item Section ==================== */
.new-item-section {
  padding: 4px;
}

.new-item-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  background: transparent;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  color: var(--color-primary);
  font-size: 0.85em;
  text-align: left;
  transition: all var(--transition-fast);
}

.new-item-btn:hover,
.new-item-btn.highlighted {
  background-color: var(--color-primary);
  color: white;
}

.new-item-btn.highlighted .plus-icon {
  color: white;
}

.plus-icon {
  font-weight: 600;
  font-size: 1.1em;
  color: var(--color-primary);
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
