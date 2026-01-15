// tests/unit/sum.test.ts
import { describe, it, expect } from 'vitest';
import { sum } from '@/utils/sum';

describe('sum', () => {
  it('adds 1 + 2 to equal 3', () => {
    expect(sum(1, 2)).toBe(3);
  });
});
