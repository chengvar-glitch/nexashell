<template>
  <MetadataMultiSelect
    :model-value="modelValue"
    :items="groups"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createGroupText"
    :empty-text="emptyText"
    :allow-create="allowCreateGroup"
    :immediate-save="immediateSave"
    kind="group"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('group-added', item)"
  />
</template>

<script setup lang="ts">
import MetadataMultiSelect from './MetadataMultiSelect.vue';
import type { MetadataItem } from '@/core/types/common';

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

withDefaults(defineProps<Props>(), {
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
</script>
