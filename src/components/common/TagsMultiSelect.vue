<template>
  <MultiSelect
    :model-value="modelValue"
    :items="tags"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createTagText"
    :empty-text="emptyText"
    :allow-create="allowCreateTag"
    :immediate-save="immediateSave"
    item-type="tag"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('tag-added', item as Tag)"
  />
</template>

<script setup lang="ts">
import MultiSelect from './MultiSelect.vue';

interface Tag {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

interface Props {
  modelValue?: string[];
  tags?: Tag[];
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
  'tag-added': [tag: Tag];
}>();
</script>
