/**
 * Event bus utility
 * Provides type-safe event publishing and subscription mechanism
 *
 * Fixed: Proper listener tracking to prevent memory leaks
 */

import { APP_EVENTS, type AppEventType } from '@/core/constants';

type EventHandler = (...args: unknown[]) => void;
type WrappedHandler = (e: Event) => void;

/**
 * Event bus class with proper memory management
 */
class EventBus {
  /**
   * Map to track wrapped handlers for proper cleanup
   */
  private listeners = new Map<
    AppEventType,
    Map<EventHandler, WrappedHandler>
  >();

  /**
   * Emit event
   */
  emit(event: AppEventType, ...args: unknown[]): void {
    window.dispatchEvent(new CustomEvent(event, { detail: args }));
  }

  /**
   * Listen to event
   * Creates and stores wrapper to enable proper removal later
   */
  on(event: AppEventType, handler: EventHandler): void {
    // Create wrapped handler that extracts CustomEvent details
    const wrappedHandler: WrappedHandler = (e: Event) => {
      if (e instanceof CustomEvent) {
        handler(...(e.detail || []));
      }
    };

    // Track wrapped handler for this event
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Map());
    }
    this.listeners.get(event)!.set(handler, wrappedHandler);

    // Register the actual listener
    window.addEventListener(event, wrappedHandler);
  }

  /**
   * Remove event listener
   * Uses stored wrapper to properly remove listener
   */
  off(event: AppEventType, handler: EventHandler): void {
    const eventListeners = this.listeners.get(event);
    if (!eventListeners) {
      console.warn(`[EventBus] No listeners found for event: ${event}`);
      return;
    }

    const wrappedHandler = eventListeners.get(handler);
    if (!wrappedHandler) {
      console.warn(`[EventBus] Listener not found for event: ${event}`);
      return;
    }

    // Remove actual listener with stored wrapper
    window.removeEventListener(event, wrappedHandler);

    // Clean up tracking
    eventListeners.delete(handler);
    if (eventListeners.size === 0) {
      this.listeners.delete(event);
    }
  }

  /**
   * Remove all listeners for an event
   */
  offAll(event: AppEventType): void {
    const eventListeners = this.listeners.get(event);
    if (!eventListeners) return;

    eventListeners.forEach(wrappedHandler => {
      window.removeEventListener(event, wrappedHandler);
    });

    this.listeners.delete(event);
  }

  /**
   * Get count of listeners for debugging
   */
  getListenerCount(event?: AppEventType): number {
    if (event) {
      return this.listeners.get(event)?.size ?? 0;
    }
    return Array.from(this.listeners.values()).reduce(
      (total, map) => total + map.size,
      0
    );
  }
}

export const eventBus = new EventBus();

// Convenient method exports
export { APP_EVENTS };
