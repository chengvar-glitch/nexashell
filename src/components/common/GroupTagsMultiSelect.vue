<template>
  <MultiSelect
    :model-value="modelValue"
    :items="groups"
    :label="label"
    :placeholder="placeholder"
    :create-item-text="createGroupText"
    :empty-text="emptyText"
    :allow-create="allowCreateGroup"
    :immediate-save="immediateSave"
    item-type="group"
    @update:model-value="val => emit('update:modelValue', val)"
    @item-added="item => emit('group-added', item as Group)"
  />
</template>

<script setup lang="ts">
import MultiSelect from './MultiSelect.vue';

interface Group {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}

interface Props {
  modelValue?: string[];
  groups?: Group[];
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
  'group-added': [group: Group];
}>();
</script>
