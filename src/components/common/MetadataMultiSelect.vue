<template>
  <div class="metadata-multi-select">
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
      @create-error="createError = $event"
    />
    <p v-if="createError" class="create-error" role="alert">
      {{ createError }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import MultiSelect from './MultiSelect.vue';
import type { MetadataItem } from '@/core/types/common';
import { eventBus } from '@/core/utils';
import { APP_EVENTS } from '@/core/constants';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('METADATA_MULTI_SELECT');
const { t } = useI18n();

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

const createError = ref<string>('');

const handleCreateItem = async (name: string): Promise<MetadataItem> => {
  createError.value = '';
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
      logger.error(`Failed to create ${props.kind}:`, error);
      // Surface the backend failure to the user instead of failing silently.
      const msg = error instanceof Error ? error.message : String(error);
      createError.value = msg || t('metadata.createFailed');
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

<style scoped>
.create-error {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--color-danger);
  word-break: break-word;
}
</style>
