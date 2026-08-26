import { describe, it, expect } from 'vitest';
import { ref } from 'vue';
import { parentOfPath, normalizePath, useSftp } from './use-sftp';

describe('normalizePath', () => {
  it('defaults empty paths to root', () => {
    expect(normalizePath('')).toBe('/');
    expect(normalizePath('.')).toBe('/');
  });

  it('adds a leading slash to relative paths', () => {
    expect(normalizePath('home')).toBe('/home');
    expect(normalizePath('a/b')).toBe('/a/b');
  });

  it('keeps absolute paths intact', () => {
    expect(normalizePath('/a/b')).toBe('/a/b');
    expect(normalizePath('/')).toBe('/');
  });

  it('normalizes Windows backslash paths into the SFTP /C:/ form', () => {
    expect(normalizePath('C:\\Users\\dev')).toBe('/C:/Users/dev');
    expect(normalizePath('C:/Users/dev')).toBe('/C:/Users/dev');
    expect(normalizePath('D:\\')).toBe('/D:/');
  });

  it('keeps OpenSSH virtual drive paths as-is', () => {
    expect(normalizePath('/C:/Users/dev')).toBe('/C:/Users/dev');
    expect(normalizePath('/C:/')).toBe('/C:/');
  });
});

describe('parentOfPath', () => {
  it('returns null at root', () => {
    expect(parentOfPath('/')).toBeNull();
  });

  it('returns "/" for a top-level child', () => {
    expect(parentOfPath('/etc')).toBe('/');
  });

  it('returns the immediate parent for nested paths', () => {
    expect(parentOfPath('/a/b/c')).toBe('/a/b');
    expect(parentOfPath('/usr/local')).toBe('/usr');
  });

  it('handles Windows drive roots', () => {
    expect(parentOfPath('/C:/Users/dev')).toBe('/C:/Users');
    expect(parentOfPath('/C:/Users')).toBe('/C:/');
    expect(parentOfPath('/C:/')).toBe('/');
    expect(parentOfPath('C:\\Users\\dev')).toBe('/C:/Users');
  });
});

describe('useSftp return surface', () => {
  it('exposes the methods SftpBrowser relies on', () => {
    const sftp = useSftp(ref('sid'));
    // These are called directly by SftpBrowser.vue; a rename here would throw
    // "sftp.go is not a function" at runtime, so assert the full surface.
    expect(typeof sftp.go).toBe('function');
    expect(typeof sftp.goUp).toBe('function');
    expect(typeof sftp.goHome).toBe('function');
    expect(typeof sftp.refresh).toBe('function');
    expect(typeof sftp.mkdir).toBe('function');
    expect(typeof sftp.remove).toBe('function');
    expect(typeof sftp.rename).toBe('function');
    expect(typeof sftp.dispose).toBe('function');
  });

  it('exposes reactive browsing state', () => {
    const sftp = useSftp(ref('sid'));
    expect(typeof sftp.currentPath).toBe('object'); // a ref
    expect(typeof sftp.entries).toBe('object');
    expect(typeof sftp.loading).toBe('object');
    expect(typeof sftp.error).toBe('object');
  });
});