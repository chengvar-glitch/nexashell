<template>
  <MetadataMultiSelect
    :model-value="modelValue"
    :items="tags"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createTagText"
    :empty-text="emptyText"
    :allow-create="allowCreateTag"
    :immediate-save="immediateSave"
    kind="tag"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('tag-added', item)"
  />
</template>

<script setup lang="ts">
import MetadataMultiSelect from './MetadataMultiSelect.vue';
import type { MetadataItem } from '@/core/types/common';

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

withDefaults(defineProps<Props>(), {
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
</script>
