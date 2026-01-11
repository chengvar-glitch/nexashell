/**
 * 窗口操作工具函数
 * 封装与 Tauri 窗口控制相关的操作
 */

import { safeInvoke } from '@/utils/error-handler';

/**
 * 退出应用
 */
export async function quitApp(): Promise<void> {
  await safeInvoke('quit_app');
}

/**
 * 切换窗口最大化状态
 */
export async function toggleMaximize(): Promise<void> {
  await safeInvoke('toggle_maximize');
}

/**
 * 最小化窗口
 */
export async function minimizeWindow(): Promise<void> {
  await safeInvoke('minimize_window');
}

/**
 * 关闭窗口
 */
export async function closeWindow(): Promise<void> {
  await safeInvoke('close_window');
}
