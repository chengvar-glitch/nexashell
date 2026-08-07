<script setup lang="ts">
import { inject, provide, computed } from 'vue';
import SplitRenderer from '@/components/layout/SplitRenderer.ts';
import { TAB_MANAGEMENT_KEY, PANE_SIZES_COMMIT_KEY } from '@/core/types';
import type { Tab, SplitNode } from '@/features/tabs';

const props = defineProps<{
  tab: Tab;
}>();

const tabManagement = inject(TAB_MANAGEMENT_KEY);
if (!tabManagement) {
  throw new Error('TAB_MANAGEMENT_KEY must be provided');
}

const activePaneId = tabManagement.activePaneId;
const setActivePane = tabManagement.setActivePane;

/**
 * Writes dragged sizes back into the underlying split tree. Receives the
 * node object reference from the emitting SplitRenderer and mutates it in
 * place — the node IS part of tab.splitTree (deep object identity), so this
 * persists the layout without touching any props directly.
 */
provide(PANE_SIZES_COMMIT_KEY, (node: SplitNode, sizes: number[]) => {
  if (node.kind === 'split' && sizes.length === node.children.length) {
    node.sizes = [...sizes];
  }
});

/**
 * Single render path for both single-pane and split-pane tabs: when there's
 * no split tree we synthesize a one-pane tree. SplitRenderer keys leaf panes
 * by their stable pane id, so splitting/collapsing never unmounts an
 * existing pane's RemoteConnectionView — its xterm history survives.
 */
const renderNode = computed<SplitNode | null>(() => {
  if (props.tab.splitTree) return props.tab.splitTree;
  const pane = props.tab.panes?.[0];
  if (!pane) return null;
  return { kind: 'pane', paneId: pane.id, connect: pane.connect };
});

const handlePaneClick = (paneId: string) => {
  if (!paneId) return;
  setActivePane(paneId);
};
</script>

<template>
  <div class="pane-root">
    <SplitRenderer
      v-if="renderNode"
      :node="renderNode"
      :tab-type="tab.type"
      :active-pane-id="activePaneId"
      @pane-click="handlePaneClick"
    />
  </div>
</template>

<style scoped>
.pane-root {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: var(--color-bg-secondary);
}
</style>

<!-- Global (non-scoped): SplitRenderer's render-function vnodes don't carry
     this component's scope id, so scoped rules can't reach them. -->
<style>
.pane-root .split-bar:hover {
  background-color: var(--color-primary, #facc15) !important;
}
</style>
