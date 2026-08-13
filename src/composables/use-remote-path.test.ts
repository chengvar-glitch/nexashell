import { describe, it, expect } from 'vitest';
import {
  normalizeRemotePath,
  resolveRelativeCd,
} from './use-remote-path';

describe('normalizeRemotePath', () => {
  it('collapses `.` segments', () => {
    expect(normalizeRemotePath('/a/./b')).toBe('/a/b');
  });

  it('collapses `..` segments', () => {
    expect(normalizeRemotePath('/a/b/../c')).toBe('/a/c');
  });

  it('clamps `..` above root to the same dir', () => {
    expect(normalizeRemotePath('/a/../..')).toBe('/');
  });

  it('removes redundant slashes and trailing slash', () => {
    expect(normalizeRemotePath('/a//b/')).toBe('/a/b');
  });

  it('leaves relative paths unchanged', () => {
    expect(normalizeRemotePath('a/b')).toBe('a/b');
  });

  it('returns "/" for the root path', () => {
    expect(normalizeRemotePath('/')).toBe('/');
  });
});

describe('resolveRelativeCd', () => {
  const HOME = '/home/user';

  it('returns empty for empty input', () => {
    expect(resolveRelativeCd('', '/base', HOME)).toBe('');
  });

  it('returns empty for `-` (previous dir unsupported)', () => {
    expect(resolveRelativeCd('-', '/base', HOME)).toBe('');
  });

  it('resolves `.` to the base path', () => {
    expect(resolveRelativeCd('.', '/base', HOME)).toBe('/base');
  });

  it('resolves `..` to the parent of base', () => {
    expect(resolveRelativeCd('..', '/base/child', HOME)).toBe('/base');
  });

  it('resolves `~` to home', () => {
    expect(resolveRelativeCd('~', '/base', HOME)).toBe('/home/user');
  });

  it('resolves `~/x` against home', () => {
    expect(resolveRelativeCd('~/x/y', '/base', HOME)).toBe('/home/user/x/y');
  });

  it('resolves a relative name against base', () => {
    expect(resolveRelativeCd('sub', '/base/child', HOME)).toBe('/base/child/sub');
  });

  it('returns empty when base is missing and path is relative', () => {
    expect(resolveRelativeCd('sub', '', HOME)).toBe('');
  });
});
