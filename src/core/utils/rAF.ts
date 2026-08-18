/**
 * requestAnimationFrame helpers for coalescing high-frequency layout work.
 *
 * Terminal `fit()` (and similar measure-then-apply work) is often triggered by
 * several sources inside the same animation frame — a window resize event and
 * the ResizeObserver it causes, CSS-transition layout churn (the terminal
 * container animates its margin when side panels open/close), tab activation,
 * etc. Multiple triggers in one frame all observe the same intermediate
 * layout, so running the work more than once per frame is pure waste. These
 * helpers guarantee at most one execution per frame.
 */

export type RafThrottled<A extends unknown[]> = ((...args: A) => void) & {
  /** Cancel a queued (not yet executed) invocation, if any. */
  cancel: () => void;
};

/**
 * Returns a function that forwards its arguments to `fn` at most once per
 * animation frame. When called several times between frames, the LAST call's
 * arguments win and the trailing invocation runs on the next rAF. Safe to call
 * any number of times; the first call of a frame schedules the execution.
 */
export function rafThrottle<A extends unknown[]>(
  fn: (...args: A) => void
): RafThrottled<A> {
  let rafId = 0;
  let lastArgs: A | null = null;

  const flush = () => {
    rafId = 0;
    if (lastArgs) {
      const args = lastArgs;
      lastArgs = null;
      fn(...args);
    }
  };

  const throttled = ((...args: A) => {
    lastArgs = args;
    // If no frame is scheduled yet, schedule one. Subsequent calls in the
    // same frame only update `lastArgs`.
    if (rafId === 0) {
      rafId = requestAnimationFrame(flush);
    }
  }) as RafThrottled<A>;

  throttled.cancel = () => {
    if (rafId !== 0) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    lastArgs = null;
  };

  return throttled;
}