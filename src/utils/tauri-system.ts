import { invoke } from '@tauri-apps/api/core';

export async function getPlatform(): Promise<string> {
  return await invoke<string>('get_platform');
}

export async function getArch(): Promise<string> {
  return await invoke<string>('get_arch');
}

export async function isMacOS(): Promise<boolean> {
  return await invoke<boolean>('is_macos');
}

export async function isWindows(): Promise<boolean> {
  return await invoke<boolean>('is_windows');
}

export async function isLinux(): Promise<boolean> {
  return await invoke<boolean>('is_linux');
}
