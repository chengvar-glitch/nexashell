/**
 * 标签页常量定义
 */

export const DEFAULT_TAB = {
  ID: 'nexashell-default',
  LABEL: 'NEXASHELL',
  TYPE: 'home' as const,
} as const;

export const TAB_LABEL_PREFIX = {
  TERMINAL: 'Terminal',
  LOCAL_TERMINAL: 'Local Terminal',
  SSH: 'SSH',
} as const;
