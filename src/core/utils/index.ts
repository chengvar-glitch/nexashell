/**
 * Utility functions export entry
 */

// Platform detection
export * from './platform/platform-detection';

// Error handling
export { AppError, safeInvoke, logError } from './error-handler';

// Event bus
export { eventBus } from './event-bus';

// Logger
export * from './logger';

// Shortcut Manager
export { ShortcutManager } from './shortcut-manager';

// Theme Manager
export { themeManager } from './theme-manager';
