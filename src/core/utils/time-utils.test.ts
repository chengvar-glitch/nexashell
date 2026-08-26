import { describe, it, expect } from 'vitest';
import { sortSessions } from './time-utils';

interface Session {
  id: string;
  updated_at: string;
  is_pinned: boolean;
  pinned_at?: string | null;
}

const s = (id: string, updated_at: string, over: Partial<Session> = {}): Session => ({
  id,
  updated_at,
  is_pinned: false,
  pinned_at: null,
  ...over,
});

describe('sortSessions', () => {
  it('puts pinned sessions first, newest pin on top, then by recency', () => {
    const list = [
      s('newest', '2026-08-03 10:00:00'),
      s('older', '2026-08-02 10:00:00'),
      s('pinned-old', '2026-08-01 10:00:00', {
        is_pinned: true,
        pinned_at: '2026-08-01 09:00:00',
      }),
      s('pinned-new', '2026-08-01 08:00:00', {
        is_pinned: true,
        pinned_at: '2026-08-01 11:00:00',
      }),
    ];
    expect(sortSessions(list).map(x => x.id)).toEqual([
      'pinned-new',
      'pinned-old',
      'newest',
      'older',
    ]);
    // Original array untouched.
    expect(list[0].id).toBe('newest');
  });
});
