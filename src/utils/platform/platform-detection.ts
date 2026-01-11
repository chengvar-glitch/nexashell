/**
 * 平台检测工具函数
 */

import { safeInvoke } from '@/utils/error-handler';

/**
 * 检测是否为 macOS 系统（通过 Tauri）
 */
export async function isMacOS(): Promise<boolean> {
  return await safeInvoke<boolean>('is_macos');
}

/**
 * 检测是否为 Windows 系统（通过 Tauri）
 */
export async function isWindows(): Promise<boolean> {
  return await safeInvoke<boolean>('is_windows');
}

/**
 * 检测是否为 Linux 系统（通过 Tauri）
 */
export async function isLinux(): Promise<boolean> {
  return await safeInvoke<boolean>('is_linux');
}

/**
 * 获取平台名称
 */
export async function getPlatform(): Promise<string> {
  return await safeInvoke<string>('get_platform');
}

/**
 * 获取系统架构
 */
export async function getArch(): Promise<string> {
  return await safeInvoke<string>('get_arch');
}

/**
 * 检测是否为 macOS 系统（浏览器环境）
 */
export function isMacOSBrowser(): boolean {
  if (typeof window !== 'undefined') {
    // 检查是否在 Tauri 环境中
    if ((window as any).__TAURI__) {
      try {
        return (window as any).__TAURI__.os.platform() === 'darwin';
      } catch (error) {
        return navigator.userAgent.includes('Mac');
      }
    }
  }
  return typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac');
}

/**
 * 检测是否为 Windows 系统（浏览器环境）
 */
export function isWindowsBrowser(): boolean {
  if (typeof window !== 'undefined') {
    // 检查是否在 Tauri 环境中
    if ((window as any).__TAURI__) {
      try {
        return (window as any).__TAURI__.os.platform() === 'windows';
      } catch (error) {
        return navigator.userAgent.includes('Windows');
      }
    }
  }
  return typeof navigator !== 'undefined' && navigator.userAgent.includes('Windows');
}
