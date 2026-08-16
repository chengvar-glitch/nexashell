import { describe, it, expect, vi, beforeEach } from 'vitest';

// Module-under-test is imported dynamically so the `vi.mock` factory below is
// guaranteed to run after `holder` is initialised (avoids a hoisting TDZ).
const holder: {
  getByLabel: (...args: unknown[]) => Promise<unknown>;
  constructCalls: Array<[string, Record<string, unknown>]>;
} = {
  getByLabel: vi.fn(),
  constructCalls: [],
};

vi.mock('@tauri-apps/api/webviewWindow', () => {
  class MockWebviewWindow {
    static getByLabel = (...args: unknown[]) => holder.getByLabel(...args);
    constructor(label: string, options: { url?: string }) {
      holder.constructCalls.push([label, options]);
    }
    once = vi.fn(async () => () => {});
    show = vi.fn(async () => {});
    setFocus = vi.fn(async () => {});
  }
  return { WebviewWindow: MockWebviewWindow };
});

describe('fileManagerWindowLabel', () => {
  it('prefixes the session id', async () => {
    const { fileManagerWindowLabel } = await import('./open-file-manager');
    expect(fileManagerWindowLabel('abc-123')).toBe('file-manager-abc-123');
  });
});

describe('openFileManagerWindow', () => {
  beforeEach(async () => {
    await import('./open-file-manager');
    holder.getByLabel = vi.fn();
    holder.constructCalls.length = 0;
  });

  it('returns false for an empty session id', async () => {
    const { openFileManagerWindow } = await import('./open-file-manager');
    expect(await openFileManagerWindow('')).toBe(false);
    expect(holder.getByLabel).not.toHaveBeenCalled();
  });

  it('reuses an existing window for the same session', async () => {
    const { openFileManagerWindow } = await import('./open-file-manager');
    holder.getByLabel = vi.fn().mockResolvedValue({
      show: vi.fn(async () => {}),
      setFocus: vi.fn(async () => {}),
    });
    const result = await openFileManagerWindow('sid-1');
    expect(result).toBe(true);
    expect(holder.getByLabel).toHaveBeenCalledWith('file-manager-sid-1');
    expect(holder.constructCalls).toHaveLength(0);
  });

  it('creates a new window when none exists', async () => {
    const { openFileManagerWindow } = await import('./open-file-manager');
    holder.getByLabel = vi.fn().mockResolvedValue(null);
    const result = await openFileManagerWindow('sid-2');
    expect(result).toBe(true);
    expect(holder.constructCalls).toHaveLength(1);
    expect(holder.constructCalls[0][0]).toBe('file-manager-sid-2');
    expect(holder.constructCalls[0][1]).toMatchObject({
      url: 'filemanager.html',
      title: 'NexaShell File Manager',
      width: 920,
      height: 720,
      center: true,
      resizable: true,
    });
  });
});
