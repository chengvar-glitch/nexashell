/**
 * Logger Utility
 *
 * Centralized logging system with levels, filtering, and formatting.
 * Supports different environments and module-based filtering.
 */

// Global variable declarations for browser environment
declare global {
  interface Window {
    __logger__?: unknown;
    __LOGGER_MANAGER__?: unknown;
  }
}

export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  module: string;
  message: string;
  data?: unknown;
}

/**
 * Logger configuration
 */
export interface LoggerConfig {
  level: LogLevel;
  enableConsole: boolean;
  enableHistory: boolean;
  maxHistorySize: number;
  moduleFilters?: string[]; // Only record specified modules; if not set, record all
}

/**
 * Module Logger class
 */
class ModuleLogger {
  private config: LoggerConfig;
  private history: LogEntry[] = [];

  constructor(
    private moduleName: string,
    config: LoggerConfig
  ) {
    this.config = config;
  }

  /**
   * Get numeric value for log level (for comparison)
   */
  private getLevelValue(level: LogLevel): number {
    const levels: Record<LogLevel, number> = {
      DEBUG: 0,
      INFO: 1,
      WARN: 2,
      ERROR: 3,
    };
    return levels[level];
  }

  /**
   * Check if the log should be recorded
   */
  private shouldLog(level: LogLevel): boolean {
    // Check log level
    if (this.getLevelValue(level) < this.getLevelValue(this.config.level)) {
      return false;
    }

    // Check module filter
    if (this.config.moduleFilters && this.config.moduleFilters.length > 0) {
      return this.config.moduleFilters.some(filter =>
        this.moduleName.includes(filter)
      );
    }

    return true;
  }

  /**
   * Format log output
   */
  private formatLog(level: LogLevel, message: string): string {
    const timestamp = new Date().toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });

    const levelColors: Record<LogLevel, string> = {
      DEBUG: '\x1b[36m', // Cyan
      INFO: '\x1b[32m', // Green
      WARN: '\x1b[33m', // Yellow
      ERROR: '\x1b[31m', // Red
    };

    const reset = '\x1b[0m';
    const color = levelColors[level];
    const levelStr = level.padEnd(5);

    return `${color}[${timestamp}] [${levelStr}] [${this.moduleName}]${reset} ${message}`;
  }

  /**
   * Core logging method
   */
  private writeLog(level: LogLevel, message: string, data?: unknown): void {
    if (!this.shouldLog(level)) {
      return;
    }

    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level,
      module: this.moduleName,
      message,
      data,
    };

    // Record to history
    if (this.config.enableHistory) {
      this.history.push(entry);
      if (this.history.length > this.config.maxHistorySize) {
        this.history.shift();
      }
    }

    // Output to console
    if (this.config.enableConsole) {
      const formattedMsg = this.formatLog(level, message);
      const consoleMethod = this.getConsoleMethod(level);

      if (data !== undefined) {
        consoleMethod(formattedMsg, data);
      } else {
        consoleMethod(formattedMsg);
      }
    }
  }

  /**
   * Get corresponding console method
   */
  private getConsoleMethod(level: LogLevel): (...args: unknown[]) => void {
    switch (level) {
      case 'DEBUG':
        return console.debug;
      case 'INFO':
        return console.info;
      case 'WARN':
        return console.warn;
      case 'ERROR':
        return console.error;
      default:
        return console.log;
    }
  }

  // Public methods

  debug(message: string, data?: unknown): void {
    this.writeLog('DEBUG', message, data);
  }

  info(message: string, data?: unknown): void {
    this.writeLog('INFO', message, data);
  }

  warn(message: string, data?: unknown): void {
    this.writeLog('WARN', message, data);
  }

  error(message: string, error?: Error | string | unknown): void {
    if (error instanceof Error) {
      this.writeLog('ERROR', message, {
        name: error.name,
        message: error.message,
        stack: error.stack,
      });
    } else {
      this.writeLog('ERROR', message, error);
    }
  }

  /**
   * Get log history
   */
  getHistory(): LogEntry[] {
    return [...this.history];
  }

  /**
   * Clear log history
   */
  clearHistory(): void {
    this.history = [];
  }

  /**
   * Export logs as JSON
   */
  exportAsJSON(): string {
    return JSON.stringify(this.history, null, 2);
  }

  /**
   * Export logs as CSV
   */
  exportAsCSV(): string {
    const headers = ['Timestamp', 'Level', 'Module', 'Message', 'Data'];
    const rows = this.history.map(entry => [
      entry.timestamp,
      entry.level,
      entry.module,
      entry.message,
      JSON.stringify(entry.data || ''),
    ]);

    const csv = [
      headers.join(','),
      ...rows.map(row => row.map(cell => `"${cell}"`).join(',')),
    ].join('\n');

    return csv;
  }
}

/**
 * Global Logger Manager
 */
class LoggerManager {
  private static instance: LoggerManager;
  private config: LoggerConfig;
  private loggers: Map<string, ModuleLogger> = new Map();

  private constructor() {
    // Set default config based on environment
    const isDev = import.meta.env.DEV;

    this.config = {
      level: isDev ? 'DEBUG' : 'INFO',
      enableConsole: true,
      enableHistory: true,
      maxHistorySize: 1000,
      moduleFilters: undefined, // No filtering, record all modules
    };
  }

  /**
   * Get singleton instance
   */
  static getInstance(): LoggerManager {
    if (!LoggerManager.instance) {
      LoggerManager.instance = new LoggerManager();
    }
    return LoggerManager.instance;
  }

  /**
   * Get or create module logger
   */
  getLogger(moduleName: string): ModuleLogger {
    if (!this.loggers.has(moduleName)) {
      this.loggers.set(moduleName, new ModuleLogger(moduleName, this.config));
    }
    return this.loggers.get(moduleName)!;
  }

  /**
   * Update global configuration
   */
  updateConfig(config: Partial<LoggerConfig>): void {
    this.config = { ...this.config, ...config };

    // Recreate all loggers to apply new configuration
    this.loggers.forEach((_, moduleName) => {
      this.loggers.set(moduleName, new ModuleLogger(moduleName, this.config));
    });
  }

  /**
   * Get current configuration
   */
  getConfig(): LoggerConfig {
    return { ...this.config };
  }

  /**
   * Get all log history
   */
  getAllHistory(): LogEntry[] {
    const allLogs: LogEntry[] = [];
    this.loggers.forEach(logger => {
      allLogs.push(...logger.getHistory());
    });
    // Sort by timestamp
    return allLogs.sort(
      (a, b) =>
        new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
  }

  /**
   * Clear all log history
   */
  clearAllHistory(): void {
    this.loggers.forEach(logger => logger.clearHistory());
  }

  /**
   * Export all logs as JSON
   */
  exportAllAsJSON(): string {
    return JSON.stringify(this.getAllHistory(), null, 2);
  }

  /**
   * Export all logs as CSV
   */
  exportAllAsCSV(): string {
    const history = this.getAllHistory();
    const headers = ['Timestamp', 'Level', 'Module', 'Message', 'Data'];
    const rows = history.map(entry => [
      entry.timestamp,
      entry.level,
      entry.module,
      entry.message,
      JSON.stringify(entry.data || ''),
    ]);

    const csv = [
      headers.join(','),
      ...rows.map(row => row.map(cell => `"${cell}"`).join(',')),
    ].join('\n');

    return csv;
  }

  /**
   * Download log file
   */
  downloadLogs(format: 'json' | 'csv' = 'json'): void {
    const content =
      format === 'json' ? this.exportAllAsJSON() : this.exportAllAsCSV();

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const blob = new (window as any).Blob([content], {
      type: format === 'json' ? 'application/json' : 'text/csv',
    });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const objectUrl = (window as any).URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = objectUrl;
    link.download = `nexashell-logs-${new Date().toISOString().slice(0, 10)}.${format}`;
    link.click();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).URL.revokeObjectURL(objectUrl);
  }
}

/**
 * Usage example:
 *
 * ```typescript
 * import { createLogger } from '@/core/utils/logger';
 *
 * const logger = createLogger('SESSION_STORE');
 *
 * logger.debug('Debug message', { data: 'value' });
 * logger.info('Information message');
 * logger.warn('Warning message');
 * logger.error('Error message', new Error('...'));
 *
 * // Get log history
 * const history = logger.getHistory();
 *
 * // Export logs
 * const json = logger.exportAsJSON();
 * const csv = logger.exportAsCSV();
 * ```
 */
export function createLogger(moduleName: string): ModuleLogger {
  return LoggerManager.getInstance().getLogger(moduleName);
}

/**
 * Get global logger manager
 */
export function getLoggerManager(): LoggerManager {
  return LoggerManager.getInstance();
}

/**
 * Expose global logger manager in development environment
 */
if (import.meta.env.DEV) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).__LOGGER_MANAGER__ = LoggerManager.getInstance();
  console.log(
    '%cLogger Manager available at window.__LOGGER_MANAGER__',
    'color: #00aa00; font-weight: bold;'
  );
}
