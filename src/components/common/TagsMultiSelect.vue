<template>
  <MultiSelect
    :model-value="modelValue"
    :items="tags"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createTagText"
    :empty-text="emptyText"
    :allow-create="allowCreateTag"
    :on-create-item="handleCreateTag"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('tag-added', item)"
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
  tags?: MetadataItem[];
  label?: string;
  placeholder?: string;
  createTagText?: string;
  emptyText?: string;
  allowCreateTag?: boolean;
  immediateSave?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: () => [],
  tags: () => [],
  label: '',
  placeholder: 'Select tags...',
  createTagText: 'Create',
  emptyText: 'No tags available',
  allowCreateTag: true,
  immediateSave: true,
});

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
  'tag-added': [tag: MetadataItem];
}>();

const handleCreateTag = async (name: string): Promise<MetadataItem> => {
  if (props.immediateSave) {
    try {
      const id = await invoke<string>('add_tag', { name });
      const newTag: MetadataItem = {
        id,
        name,
        sort: 1,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      eventBus.emit(APP_EVENTS.TAGS_UPDATED);
      return newTag;
    } catch (error) {
      console.error('Failed to create tag:', error);
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
