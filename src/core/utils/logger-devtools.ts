/**
 * Logger DevTools Utilities
 *
 * Provides convenient utilities for debugging and monitoring logs in development
 */

import { getLoggerManager, LogEntry } from './logger';

interface LoggerDevTools {
  getModuleLogs: (moduleName: string) => LogEntry[];
  filterByLevel: (level: string) => LogEntry[];
  search: (keyword: string) => LogEntry[];
  recent: (count?: number) => LogEntry[];
  stats: () => {
    total: number;
    byLevel: Record<string, number>;
    byModule: Record<string, number>;
  };
  setLevel: (level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR') => void;
  setModuleFilters: (filters: string[]) => void;
  clear: () => void;
  exportJSON: () => string;
  exportCSV: () => string;
  download: (format?: 'json' | 'csv') => void;
  getAll: () => LogEntry[];
  help: () => void;
}

/**
 * Setup logger DevTools
 * Call this in your app initialization
 */
export function setupLoggerDevTools(): void {
  if (import.meta.env.DEV) {
    const manager = getLoggerManager();

    // Expose to global scope for developer use
    const devTools: LoggerDevTools = {
      /**
       * Get logs for a specific module
       * Usage: __logger__.getModuleLogs('SESSION_STORE')
       */
      getModuleLogs: (moduleName: string) => {
        const history = manager.getAllHistory();
        return history.filter(entry => entry.module === moduleName);
      },

      /**
       * Filter logs by level
       * Usage: __logger__.filterByLevel('ERROR')
       */
      filterByLevel: (level: string) => {
        const history = manager.getAllHistory();
        return history.filter(entry => entry.level === level);
      },

      /**
       * Search logs
       * Usage: __logger__.search('connection')
       */
      search: (keyword: string) => {
        const history = manager.getAllHistory();
        return history.filter(
          entry =>
            entry.message.toLowerCase().includes(keyword.toLowerCase()) ||
            JSON.stringify(entry.data)
              .toLowerCase()
              .includes(keyword.toLowerCase())
        );
      },

      /**
       * 获取最近的N条日志
       * Usage: __logger__.recent(10)
       */
      recent: (count: number = 20) => {
        const history = manager.getAllHistory();
        return history.slice(-count);
      },

      /**
       * 清空所有日志
       */
      clear: () => {
        manager.clearAllHistory();
        console.log('Logs cleared');
      },

      /**
       * 导出日志为JSON
       */
      exportJSON: () => {
        return manager.exportAllAsJSON();
      },

      /**
       * 导出日志为CSV
       */
      exportCSV: () => {
        return manager.exportAllAsCSV();
      },

      /**
       * 下载日志文件
       * Usage: __logger__.download('json') // 或 'csv'
       */
      download: (format: 'json' | 'csv' = 'json') => {
        manager.downloadLogs(format);
      },

      /**
       * 显示日志统计
       */
      stats: () => {
        const history = manager.getAllHistory();
        const stats = {
          total: history.length,
          byLevel: {
            DEBUG: history.filter(e => e.level === 'DEBUG').length,
            INFO: history.filter(e => e.level === 'INFO').length,
            WARN: history.filter(e => e.level === 'WARN').length,
            ERROR: history.filter(e => e.level === 'ERROR').length,
          },
          byModule: {} as Record<string, number>,
        };

        history.forEach(entry => {
          stats.byModule[entry.module] =
            (stats.byModule[entry.module] || 0) + 1;
        });

        return stats;
      },

      /**
       * 设置日志级别
       * Usage: __logger__.setLevel('INFO')
       */
      setLevel: (level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR') => {
        manager.updateConfig({ level });
        console.log(`Log level set to ${level}`);
      },

      /**
       * 设置模块过滤
       * Usage: __logger__.setModuleFilters(['SESSION', 'TERMINAL'])
       */
      setModuleFilters: (filters: string[]) => {
        manager.updateConfig({
          moduleFilters: filters.length > 0 ? filters : undefined,
        });
        console.log(`Module filters set to:`, filters);
      },

      /**
       * 获取所有日志
       */
      getAll: () => manager.getAllHistory(),

      /**
       * 帮助信息
       */
      help: () => {
        console.log(`
Logger DevTools Available:

Methods:
  __logger__.getModuleLogs(moduleName)  - Get logs for a specific module
  __logger__.filterByLevel(level)       - Filter logs by level (DEBUG|INFO|WARN|ERROR)
  __logger__.search(keyword)            - Search logs by keyword
  __logger__.recent(count)              - Get last N logs (default 20)
  __logger__.clear()                    - Clear all logs
  __logger__.exportJSON()               - Export logs as JSON string
  __logger__.exportCSV()                - Export logs as CSV string
  __logger__.download(format)           - Download logs (json|csv)
  __logger__.stats()                    - Show log statistics
  __logger__.setLevel(level)            - Set global log level
  __logger__.setModuleFilters(filters)  - Set module filters
  __logger__.getAll()                   - Get all logs
  __logger__.help()                     - Show this help message

Examples:
  __logger__.search('error')
  __logger__.filterByLevel('ERROR')
  __logger__.getModuleLogs('SESSION_STORE')
  __logger__.download('json')
  __logger__.setLevel('DEBUG')
        `);
      },
    };

    (window as unknown as { __logger__: LoggerDevTools }).__logger__ = devTools;

    console.log(
      '%cLogger DevTools activated! Type __logger__.help() for commands',
      'color: #00aa00; font-weight: bold;'
    );
  }
}
