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
 * Single render path for both single-pane and split-pane tabs: a single pane
 * is synthesized as a one-child SPLIT tree so SplitRenderer always recurses
 * through its stable keyed `pane-{id}` wrapper. This is what keeps the source
 * pane's RemoteConnectionView (and its xterm history) mounted when a split
 * turns that one-child tree into a two-child tree — the first child keeps the
 * same id/key, so Vue reuses the same pane instance instead of unmounting it.
 * (Rendering the single pane as a bare `{ kind: 'pane' }` node would make the
 * top-level vnode change shape on split and remount the xterm.)
 */
const renderNode = computed<SplitNode | null>(() => {
  if (props.tab.splitTree) return props.tab.splitTree;
  const pane = props.tab.panes?.[0];
  if (!pane) return null;
  return {
    kind: 'split',
    direction: 'horizontal',
    children: [{ kind: 'pane', paneId: pane.id, connect: pane.connect }],
    sizes: [100],
  };
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
