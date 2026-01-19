<template>
  <MultiSelect
    :model-value="modelValue"
    :items="groups"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createGroupText"
    :empty-text="emptyText"
    :allow-create="allowCreateGroup"
    :on-create-item="handleCreateGroup"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('group-added', item)"
  />
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import MultiSelect from './MultiSelect.vue';
import type { MetadataItem } from '@/core/types/common';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';

interface Props {
  modelValue?: string[];
  groups?: MetadataItem[];
  label?: string;
  placeholder?: string;
  createGroupText?: string;
  emptyText?: string;
  allowCreateGroup?: boolean;
  immediateSave?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: () => [],
  groups: () => [],
  label: '',
  placeholder: 'Select groups...',
  createGroupText: 'Create',
  emptyText: 'No groups available',
  allowCreateGroup: true,
  immediateSave: true,
});

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
  'group-added': [group: MetadataItem];
}>();

const handleCreateGroup = async (name: string): Promise<MetadataItem> => {
  if (props.immediateSave) {
    try {
      const id = await invoke<string>('add_group', { name });
      const newGroup: MetadataItem = {
        id,
        name,
        sort: 1,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      eventBus.emit(APP_EVENTS.GROUPS_UPDATED);
      return newGroup;
    } catch (error) {
      console.error('Failed to create group:', error);
      throw error;
    }
  } else {
    // Return a temporary item that will be saved later by the parent
    return {
      id: `new:${name}`,
      name,
      sort: 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }
};
</script>
