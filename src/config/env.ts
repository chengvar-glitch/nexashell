/**
 * Environment configuration
 * Unified management of application environment variables and configuration items
 */

export const config = {
  // Environment identifier
  isDev: import.meta.env.DEV,
  isProd: import.meta.env.PROD,
  
  // Development server configuration
  vitePort: import.meta.env.VITE_PORT || 1420,
  
  // Application information
  appVersion: import.meta.env.VITE_APP_VERSION || '0.1.0',
  appName: 'NexaShell',
  
  // Feature switches (can be controlled via environment variables)
  enableDebugMode: import.meta.env.VITE_DEBUG === 'true',
  
} as const;

/**
 * Development environment check
 */
export function isDevMode(): boolean {
  return config.isDev;
}

/**
 * Production environment check
 */
export function isProdMode(): boolean {
  return config.isProd;
}

/**
 * Debug mode check
 */
export function isDebugMode(): boolean {
  return config.enableDebugMode;
}
