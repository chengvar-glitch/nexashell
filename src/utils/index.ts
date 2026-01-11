/**
 * Utility functions export entry
 */

// Window operations
export * from './window/window-operations';

// Platform detection
export * from './platform/platform-detection';

// Tab operations
export * from './tab/tab-operations';

// Error handling
export { AppError, safeInvoke, logError } from './error-handler';

// Event bus
export { eventBus, APP_EVENTS } from './event-bus';
