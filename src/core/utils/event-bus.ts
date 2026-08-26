/**
 * Event bus utility
 * Provides type-safe event publishing and subscription mechanism
 *
 * Fixed: Proper listener tracking to prevent memory leaks
 */

import type { AppEventType } from '@/core/constants';

type EventHandler = (...args: unknown[]) => void | Promise<void>;
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
   * Creates and stores wrapper to enable proper removal later.
   * Returns an unsubscribe function so callers can clean up without keeping
   * a reference to the handler.
   *
   * Registering the same handler twice for the same event replaces the old
   * registration and unregisters the previous window listener first, so no
   * listener is leaked.
   */
  on(event: AppEventType, handler: EventHandler): () => void {
    // Create wrapped handler that extracts CustomEvent details
    const wrappedHandler: WrappedHandler = (e: Event) => {
      if (e instanceof CustomEvent) {
        try {
          // Promise.resolve also surfaces async rejections; the .catch
          // prevents an unhandled rejection from an async subscriber.
          Promise.resolve(handler(...(e.detail || []))).catch(err => {
            // A failing subscriber must not break dispatchEvent for others.
            console.error('[EventBus] async handler error', err);
          });
        } catch (err) {
          // A failing subscriber must not break dispatchEvent for others.
          console.error('[EventBus] handler error', err);
        }
      }
    };

    // If this handler was already registered, remove the old listener first
    const existing = this.listeners.get(event)?.get(handler);
    if (existing) {
      window.removeEventListener(event, existing);
    }

    // Track wrapped handler for this event
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Map());
    }
    this.listeners.get(event)!.set(handler, wrappedHandler);

    // Register the actual listener
    window.addEventListener(event, wrappedHandler);

    return () => this.off(event, handler);
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

}

export const eventBus = new EventBus();
