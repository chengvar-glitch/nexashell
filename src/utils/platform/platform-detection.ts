/**
 * Platform detection utility functions
 */

import { safeInvoke } from '@/utils/error-handler';

/**
 * Detect if it's a macOS system (via Tauri)
 */
export async function isMacOS(): Promise<boolean> {
  return await safeInvoke<boolean>('is_macos');
}

/**
 * Detect if it's a Windows system (via Tauri)
 */
export async function isWindows(): Promise<boolean> {
  return await safeInvoke<boolean>('is_windows');
}

/**
 * Detect if it's a Linux system (via Tauri)
 */
export async function isLinux(): Promise<boolean> {
  return await safeInvoke<boolean>('is_linux');
}

/**
 * Get platform name
 */
export async function getPlatform(): Promise<string> {
  return await safeInvoke<string>('get_platform');
}

/**
 * Get system architecture
 */
export async function getArch(): Promise<string> {
  return await safeInvoke<string>('get_arch');
}

/**
 * Detect if it's a macOS system (browser environment)
 */
export function isMacOSBrowser(): boolean {
  if (typeof window !== 'undefined') {
    // Check if in Tauri environment
    if ((window as any).__TAURI__) {
      try {
        return (window as any).__TAURI__.os.platform() === 'darwin';
      } catch {
        return navigator.userAgent.includes('Mac');
      }
    }
  }
  return (
    typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac')
  );
}

/**
 * Detect if it's a Windows system (browser environment)
 */
export function isWindowsBrowser(): boolean {
  if (typeof window !== 'undefined') {
    // Check if in Tauri environment
    if ((window as any).__TAURI__) {
      try {
        return (window as any).__TAURI__.os.platform() === 'windows';
      } catch {
        return navigator.userAgent.includes('Windows');
      }
    }
  }
  return (
    typeof navigator !== 'undefined' && navigator.userAgent.includes('Windows')
  );
}
