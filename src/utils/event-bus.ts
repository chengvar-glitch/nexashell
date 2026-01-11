/**
 * Event bus utility
 * Provides type-safe event publishing and subscription mechanism
 */

import { APP_EVENTS, type AppEventType } from '@/constants';

type EventHandler = (...args: any[]) => void;

/**
 * Event bus class
 */
class EventBus {
  /**
   * Emit event
   */
  emit(event: AppEventType, ...args: any[]): void {
    window.dispatchEvent(new CustomEvent(event, { detail: args }));
  }

  /**
   * Listen to event
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
   * Remove event listener
   */
  off(event: AppEventType, handler: EventHandler): void {
    window.removeEventListener(event, handler as any);
  }
}

export const eventBus = new EventBus();

// Convenient method exports
export { APP_EVENTS };
