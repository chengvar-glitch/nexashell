/**
 * Pure scroll-target computation for a horizontally scrolling tab strip.
 *
 * `tabRect` and `containerLeft` come from getBoundingClientRect() (viewport
 * coordinates), so the math is naturally scroll-aware — unlike offsetLeft,
 * which ignores ancestor scrolling and makes the left-edge case unreachable.
 */

export interface ScrollViewport {
  scrollLeft: number;
  clientWidth: number;
}

export interface TabRect {
  left: number;
  right: number;
}

/**
 * Compute the target scrollLeft that brings `tabRect` fully into view.
 * Returns null when the tab is already fully visible.
 */
export const computeScrollTarget = (
  viewport: ScrollViewport,
  tabRect: TabRect,
  containerLeft: number
): number | null => {
  const relLeft = tabRect.left - containerLeft;
  const relRight = tabRect.right - containerLeft;
  if (relLeft < 0) return viewport.scrollLeft + relLeft;
  if (relRight > viewport.clientWidth) {
    return viewport.scrollLeft + (relRight - viewport.clientWidth);
  }
  return null;
};