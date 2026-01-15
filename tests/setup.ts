import { vi } from 'vitest';
// Mock Tauri invoke
vi.mock('@tauri-apps/api', async importOriginal => {
  const actual = await importOriginal();
  return {
    ...(actual as object),
    invoke: vi.fn(),
  };
});
