<script setup lang="ts">
import { inject, provide, computed } from 'vue';
import RemoteConnectionView from '@/components/connections/RemoteConnectionView.vue';
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
  if (node.kind === 'split') {
    node.sizes = [...sizes];
  }
});

const isSinglePane = computed(() => {
  return !props.tab.splitTree && (props.tab.panes?.length ?? 0) <= 1;
});

const firstPaneId = computed(() => props.tab.panes?.[0]?.id || '');

const firstPaneConnect = computed(() => {
  const connect = props.tab.panes?.[0]?.connect;
  return connect
    ? {
        ip: connect.ip,
        port: connect.port,
        username: connect.username,
      }
    : {};
});

const handlePaneClick = (paneId: string) => {
  if (!paneId) return;
  setActivePane(paneId);
};

const getPaneClass = (paneId: string) => {
  return {
    'pane-container': true,
    'pane-active': !!paneId && activePaneId.value === paneId,
  };
};
</script>

<template>
  <div class="pane-root">
    <template v-if="isSinglePane">
      <div
        :class="getPaneClass(firstPaneId)"
        @click="handlePaneClick(firstPaneId)"
      >
        <RemoteConnectionView
          v-if="firstPaneId"
          :session-id="firstPaneId"
          :tab-type="tab.type"
          v-bind="firstPaneConnect"
        />
      </div>
    </template>
    <template v-else-if="tab.splitTree">
      <SplitRenderer
        :node="tab.splitTree"
        :tab-type="tab.type"
        :active-pane-id="activePaneId"
        @pane-click="handlePaneClick"
      />
    </template>
  </div>
</template>

<style scoped>
.pane-root {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: var(--color-bg-secondary);
}

.pane-container {
  width: 100%;
  height: 100%;
  position: relative;
  outline: none;
}

.pane-container.pane-active {
  box-shadow: inset 0 0 0 1.5px var(--color-primary, #facc15);
  z-index: 1;
}

.split-bar {
  flex-shrink: 0;
  background-color: var(--color-border-secondary, #333);
  transition: background-color 0.15s ease;
  position: relative;
  z-index: 2;
}

.split-bar:hover {
  background-color: var(--color-primary, #facc15);
}
</style>

<!-- Global (non-scoped): SplitRenderer's render-function vnodes don't carry
     this component's scope id, so scoped rules can't reach them. -->
<style>
.split-bar:hover {
  background-color: var(--color-primary, #facc15) !important;
}
</style>