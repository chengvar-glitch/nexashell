<template>
  <MultiSelect
    :model-value="modelValue"
    :items="items"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createItemText"
    :empty-text="emptyText"
    :allow-create="allowCreate"
    :on-create-item="handleCreateItem"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('item-added', item)"
  />
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import MultiSelect from './MultiSelect.vue';
import type { MetadataItem } from '@/core/types/common';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('METADATA_MULTI_SELECT');

type MetadataKind = 'group' | 'tag';

interface Props {
  modelValue?: string[];
  items?: MetadataItem[];
  label?: string;
  placeholder?: string;
  createItemText?: string;
  emptyText?: string;
  allowCreate?: boolean;
  immediateSave?: boolean;
  /** Which entity this control manages: group or tag */
  kind: MetadataKind;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: () => [],
  items: () => [],
  label: '',
  placeholder: 'Select items...',
  createItemText: 'Create',
  emptyText: 'No items available',
  allowCreate: true,
  immediateSave: true,
});

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
  'item-added': [item: MetadataItem];
}>();

const handleCreateItem = async (name: string): Promise<MetadataItem> => {
  if (props.immediateSave) {
    try {
      const addCmd = props.kind === 'group' ? 'add_group' : 'add_tag';
      const id = await invoke<string>(addCmd, { name });
      const newItem: MetadataItem = {
        id,
        name,
        sort: 1,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      eventBus.emit(
        props.kind === 'group' ? APP_EVENTS.GROUPS_UPDATED : APP_EVENTS.TAGS_UPDATED
      );
      return newItem;
    } catch (error) {
      logger.error(
        `Failed to create ${props.kind}:`,
        error
      );
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
