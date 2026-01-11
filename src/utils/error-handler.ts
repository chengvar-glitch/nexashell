/**
 * 错误处理工具
 * 提供统一的错误处理和 Tauri invoke 封装
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * 应用错误类
 */
export class AppError extends Error {
  constructor(
    message: string,
    public code: string,
    public originalError?: unknown
  ) {
    super(message);
    this.name = 'AppError';
  }
}

/**
 * 安全的 Tauri invoke 调用
 * 统一处理错误并提供类型安全
 */
export async function safeInvoke<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const appError = new AppError(
      `Failed to invoke ${command}`,
      `INVOKE_ERROR_${command.toUpperCase()}`,
      error
    );
    console.error(appError);
    throw appError;
  }
}

/**
 * 错误日志记录
 */
export function logError(error: Error | AppError, context?: string): void {
  const errorInfo = {
    name: error.name,
    message: error.message,
    context,
    timestamp: new Date().toISOString(),
  };

  if (error instanceof AppError) {
    console.error('AppError:', errorInfo, error.originalError);
  } else {
    console.error('Error:', errorInfo);
  }
}
