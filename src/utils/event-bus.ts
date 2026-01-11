/**
 * 事件总线工具
 * 提供类型安全的事件发布订阅机制
 */

import { APP_EVENTS, type AppEventType } from '@/constants';

type EventHandler = (...args: any[]) => void;

/**
 * 事件总线类
 */
class EventBus {
  /**
   * 发送事件
   */
  emit(event: AppEventType, ...args: any[]): void {
    window.dispatchEvent(new CustomEvent(event, { detail: args }));
  }

  /**
   * 监听事件
   */
  on(event: AppEventType, handler: EventHandler): void {
    const wrappedHandler = (e: Event) => {
      if (e instanceof CustomEvent) {
        handler(...(e.detail || []));
      }
    };
    window.addEventListener(event, wrappedHandler);
  }

  /**
   * 取消监听事件
   */
  off(event: AppEventType, handler: EventHandler): void {
    window.removeEventListener(event, handler as any);
  }
}

export const eventBus = new EventBus();

// 便捷方法导出
export { APP_EVENTS };
