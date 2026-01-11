/**
 * Error handling utility
 * Provides unified error handling and Tauri invoke wrapper
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Application error class
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
 * Safe Tauri invoke call
 * Unifies error handling and provides type safety
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
 * Error logging
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
