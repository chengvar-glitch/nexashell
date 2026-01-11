/**
 * 环境配置
 * 统一管理应用的环境变量和配置项
 */

export const config = {
  // 环境标识
  isDev: import.meta.env.DEV,
  isProd: import.meta.env.PROD,
  
  // 开发服务器配置
  vitePort: import.meta.env.VITE_PORT || 1420,
  
  // 应用信息
  appVersion: import.meta.env.VITE_APP_VERSION || '0.1.0',
  appName: 'NexaShell',
  
  // 功能开关（可通过环境变量控制）
  enableDebugMode: import.meta.env.VITE_DEBUG === 'true',
  
} as const;

/**
 * 开发环境检查
 */
export function isDevMode(): boolean {
  return config.isDev;
}

/**
 * 生产环境检查
 */
export function isProdMode(): boolean {
  return config.isProd;
}

/**
 * 调试模式检查
 */
export function isDebugMode(): boolean {
  return config.enableDebugMode;
}
