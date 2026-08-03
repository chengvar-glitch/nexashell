import { defineComponent, h, ref, watch, inject, onUnmounted, type VNode } from 'vue';
import type { PropType } from 'vue';
import RemoteConnectionView from '@/components/connections/RemoteConnectionView.vue';
import { PANE_SIZES_COMMIT_KEY } from '@/core/types';
import type { SplitNode } from '@/features/tabs';

const SplitRenderer = defineComponent({
  name: 'SplitRenderer',
  props: {
    node: {
      type: Object as PropType<SplitNode>,
      required: true,
    },
    tabType: {
      type: String,
      required: true,
    },
    activePaneId: {
      type: String,
      required: true,
    },
  },
  emits: ['pane-click'],
  setup(props, { emit }) {
    // Provided by PaneContainer — commits dragged sizes into the tree
    // (mutating props.node.sizes directly trips vue/no-mutating-props)
    const commitSizes = inject(PANE_SIZES_COMMIT_KEY);
    // Local reactive copy of sizes — mutating props.node.sizes directly does
    // NOT trigger re-render (props are shallow-reactive), which would make
    // drag-resize appear frozen.
    const localSizes = ref<number[]>(
      props.node.kind === 'split' ? [...props.node.sizes] : []
    );
    watch(
      () => (props.node.kind === 'split' ? props.node.sizes : null),
      sizes => {
        if (sizes) localSizes.value = [...sizes];
      }
    );

    const dragState = ref<{
      startX: number;
      startY: number;
      startSizes: number[];
      index: number;
      containerSize: number;
    } | null>(null);

    const containerRef = ref<HTMLElement | null>(null);

    const handlePointerDown = (e: MouseEvent, index: number) => {
      e.preventDefault();
      const container = containerRef.value;
      if (!container) return;
      if (props.node.kind !== 'split') return;
      const rect = container.getBoundingClientRect();
      const isHorizontal = props.node.direction === 'horizontal';
      const containerSize = isHorizontal ? rect.width : rect.height;
      dragState.value = {
        startX: e.clientX,
        startY: e.clientY,
        startSizes: [...localSizes.value],
        index,
        containerSize,
      };
      document.addEventListener('pointermove', handlePointerMove);
      document.addEventListener('pointerup', handlePointerUp);
    };

    const handlePointerMove = (e: MouseEvent) => {
      const ds = dragState.value;
      if (!ds) return;
      if (props.node.kind !== 'split') return;
      const isHorizontal = props.node.direction === 'horizontal';
      const delta = isHorizontal ? e.clientX - ds.startX : e.clientY - ds.startY;
      const deltaPercent = (delta / ds.containerSize) * 100;
      const newSizes = [...ds.startSizes];
      const leftIdx = ds.index;
      const rightIdx = ds.index + 1;
      if (leftIdx >= newSizes.length || rightIdx >= newSizes.length) return;
      const newLeft = Math.max(10, Math.min(90, ds.startSizes[leftIdx] + deltaPercent));
      const newRight = Math.max(10, Math.min(90, ds.startSizes[rightIdx] - deltaPercent));
      newSizes[leftIdx] = newLeft;
      newSizes[rightIdx] = newRight;
      const total = newSizes.reduce((a, b) => a + b, 0);
      localSizes.value = newSizes.map(s => (s / total) * 100);
    };

    const handlePointerUp = () => {
      // Commit dragged sizes back to the tree so the layout survives
      // tab-switch remounts and subsequent split/collapse operations
      if (dragState.value && props.node.kind === 'split') {
        commitSizes?.(props.node, [...localSizes.value]);
      }
      dragState.value = null;
      document.removeEventListener('pointermove', handlePointerMove);
      document.removeEventListener('pointerup', handlePointerUp);
    };

    onUnmounted(() => {
      document.removeEventListener('pointermove', handlePointerMove);
      document.removeEventListener('pointerup', handlePointerUp);
    });

    const renderPane = (node: SplitNode): VNode | null => {
      if (node.kind === 'pane') {
        return h('div', {
          class: {
            'pane-container': true,
            'pane-active': props.activePaneId === node.paneId,
          },
          onClick: () => emit('pane-click', node.paneId),
        }, [
          h(RemoteConnectionView, {
            'session-id': node.paneId,
            'tab-type': props.tabType,
          }),
        ]);
      }
      return null;
    };

    const renderSplit = (node: SplitNode): VNode | null => {
      if (node.kind !== 'split') return renderPane(node);
      const isHorizontal = node.direction === 'horizontal';
      const children: VNode[] = [];
      node.children.forEach((child, index) => {
        const isLast = index === node.children.length - 1;
        const size = localSizes.value[index] ?? 100 / node.children.length;
        // Key by stable identity (kind + paneId) instead of raw index — after
        // a collapse/expand the same index can map to a different subtree, and
        // Vue would reuse the wrong instance (leaking stale localSizes)
        const childKey =
          child.kind === 'pane'
            ? `pane-${child.paneId}`
            : `split-${index}`;
        const childEl = h(SplitRenderer, {
          key: childKey,
          node: child,
          tabType: props.tabType,
          activePaneId: props.activePaneId,
          onPaneClick: (paneId: string) => emit('pane-click', paneId),
        });

        const barWidth = 4;
        const childStyle = isHorizontal
          ? { width: `calc(${size}% - ${isLast ? 0 : barWidth}px)`, height: '100%', flexShrink: 0, overflow: 'hidden' }
          : { height: `calc(${size}% - ${isLast ? 0 : barWidth}px)`, width: '100%', flexShrink: 0, overflow: 'hidden' };

        children.push(h('div', { style: childStyle, key: `child-${index}` }, childEl));

        if (!isLast) {
          const barStyle = isHorizontal
            ? { width: `${barWidth}px`, height: '100%', cursor: 'col-resize' }
            : { height: `${barWidth}px`, width: '100%', cursor: 'row-resize' };
          children.push(h('div', {
            key: `bar-${index}`,
            class: 'split-bar',
            style: barStyle,
            onPointerdown: (e: MouseEvent) => handlePointerDown(e, index),
          }));
        }
      });

      const containerStyle = isHorizontal
        ? { display: 'flex', flexDirection: 'row' as const, width: '100%', height: '100%', overflow: 'hidden' }
        : { display: 'flex', flexDirection: 'column' as const, width: '100%', height: '100%', overflow: 'hidden' };

      return h('div', {
        ref: containerRef,
        style: containerStyle,
      }, children);
    };

    return () => {
      const node = props.node;
      if (node.kind === 'pane') {
        return renderPane(node);
      }
      return renderSplit(node);
    };
  },
});

export default SplitRenderer;