import { describe, it, expect } from 'vitest';
import { computeScrollTarget } from './tab-scroll';

const viewport = (scrollLeft: number, clientWidth: number) => ({
  scrollLeft,
  clientWidth,
});

describe('computeScrollTarget', () => {
  it('returns null when the tab is fully visible', () => {
    expect(computeScrollTarget(viewport(0, 800), { left: 100, right: 220 }, 0)).toBeNull();
    expect(computeScrollTarget(viewport(500, 800), { left: 600, right: 720 }, 0)).toBeNull();
  });

  it('scrolls right when the tab sticks out past the right edge', () => {
    // scrollLeft 0: tab ends at 1300 in an 800px viewport -> scroll to 500
    expect(computeScrollTarget(viewport(0, 800), { left: 1180, right: 1300 }, 0)).toBe(500);
    // already scrolled 300: same layout position -> 300 + 500
    expect(computeScrollTarget(viewport(300, 800), { left: 1180, right: 1300 }, 0)).toBe(800);
  });

  it('scrolls left when the tab sticks out past the left edge', () => {
    // tab starts 60px before the container's left edge -> scroll back by 60
    expect(computeScrollTarget(viewport(900, 800), { left: 840, right: 960 }, 900)).toBe(840);
    // a container that starts at x=800 on screen shifts both edges equally
    expect(computeScrollTarget(viewport(0, 800), { left: 740, right: 860 }, 800)).toBe(-60);
  });

  it('aligns the right edge of a tab wider than the viewport', () => {
    expect(computeScrollTarget(viewport(0, 200), { left: 0, right: 400 }, 0)).toBe(200);
  });
});