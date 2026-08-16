import { describe, it, expect } from 'vitest';
import { compareVersions, isNewerVersion } from './version';

describe('compareVersions', () => {
  it('returns 0 for equal versions', () => {
    expect(compareVersions('1.10.2', '1.10.2')).toBe(0);
    expect(compareVersions('v1.11.0', '1.11.0')).toBe(0);
  });

  it('compares major versions', () => {
    expect(compareVersions('2.0.0', '1.9.9')).toBe(1);
    expect(compareVersions('1.9.9', '2.0.0')).toBe(-1);
  });

  it('compares minor versions numerically', () => {
    expect(compareVersions('1.10.0', '1.9.9')).toBe(1);
    expect(compareVersions('1.9.9', '1.10.0')).toBe(-1);
  });

  it('compares patch versions', () => {
    expect(compareVersions('1.10.2', '1.10.1')).toBe(1);
    expect(compareVersions('1.10.1', '1.10.2')).toBe(-1);
  });

  it('treats missing components as zero', () => {
    expect(compareVersions('1.10', '1.10.0')).toBe(0);
    expect(compareVersions('1.10.1', '1.10')).toBe(1);
  });

  it('sorts pre-release suffixes before the final release', () => {
    expect(compareVersions('1.0.0-beta.1', '1.0.0')).toBe(-1);
    expect(compareVersions('1.0.0-beta.2', '1.0.0-beta.1')).toBe(1);
  });
});

describe('isNewerVersion', () => {
  it('returns true only when latest is strictly newer', () => {
    expect(isNewerVersion('1.11.0', '1.10.2')).toBe(true);
    expect(isNewerVersion('1.10.2', '1.10.2')).toBe(false);
    expect(isNewerVersion('1.9.0', '1.10.2')).toBe(false);
  });
});
