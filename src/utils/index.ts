/**
 * 工具函数导出入口
 */

// 窗口操作
export * from './window/window-operations';

// 平台检测
export * from './platform/platform-detection';

// 标签操作
export * from './tab/tab-operations';

// 错误处理
export { AppError, safeInvoke, logError } from './error-handler';

// 事件总线
export { eventBus, APP_EVENTS } from './event-bus';
