/**
 * 标签操作工具函数
 */

import { APP_EVENTS } from '@/constants';

/**
 * 创建新的本地终端标签
 */
export function createNewLocalTab(): void {
  window.dispatchEvent(new CustomEvent(APP_EVENTS.NEW_LOCAL_TAB));
}

/**
 * 聚焦搜索框
 */
export function focusSearch(): void {
  window.dispatchEvent(new CustomEvent(APP_EVENTS.FOCUS_SEARCH));
}
