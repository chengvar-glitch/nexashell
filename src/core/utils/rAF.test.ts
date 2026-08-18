import { describe, it, expect, vi, afterEach } from 'vitest';
import { rafThrottle } from './rAF';

/**
 * Deterministic requestAnimationFrame stub: queued callbacks only run when the
 * test explicitly flushes them, so the tests do not depend on the host
 * environment's frame timing (happy-dom included).
 */
function stubRaf() {
  let cbs: FrameRequestCallback[] = [];
  let id = 0;
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
    cbs.push(cb);
    return ++id;
  });
  vi.stubGlobal('cancelAnimationFrame', () => {
    // no-op; cancellation is observable through `pending` and the flush below
  });
  return {
    flush: () => {
      const batch = cbs;
      cbs = [];
      batch.forEach(cb => cb(performance.now()));
    },
    get pending() {
      return cbs.length;
    },
  };
}

describe('rafThrottle', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('defers execution to the next animation frame', () => {
    const raf = stubRaf();
    let calls = 0;
    const throttled = rafThrottle(() => {
      calls += 1;
    });

    throttled();
    expect(calls).toBe(0);
    expect(raf.pending).toBe(1);

    raf.flush();
    expect(calls).toBe(1);
    expect(raf.pending).toBe(0);
  });

  it('coalesces multiple calls into one per frame, keeping the last args', () => {
    const raf = stubRaf();
    const seen: number[] = [];
    const throttled = rafThrottle((n: number) => seen.push(n));

    throttled(1);
    throttled(2);
    throttled(3);
    expect(seen).toEqual([]);
    expect(raf.pending).toBe(1);

    raf.flush();
    expect(seen).toEqual([3]);
    expect(raf.pending).toBe(0);

    // A later call schedules a fresh frame on its own.
    throttled(4);
    raf.flush();
    expect(seen).toEqual([3, 4]);
  });

  it('does nothing when never invoked', () => {
    const raf = stubRaf();
    const fn = vi.fn();
    rafThrottle(fn);
    expect(fn).not.toHaveBeenCalled();
    expect(raf.pending).toBe(0);
  });

  it('cancel() drops a queued invocation', () => {
    const raf = stubRaf();
    const seen: number[] = [];
    const throttled = rafThrottle((n: number) => seen.push(n));

    throttled(1);
    throttled.cancel();
    raf.flush();

    expect(seen).toEqual([]);
    // After cancellation the next call schedules a fresh frame.
    throttled(2);
    raf.flush();
    expect(seen).toEqual([2]);
  });
});