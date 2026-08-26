/**
 * Tab constants definition
 */

export const DEFAULT_TAB = {
  ID: 'nexashell-default',
  LABEL: 'NEXASHELL',
  TYPE: 'home' as const,
} as const;

/** Hard cap on how many panes a single tab may contain. */
export const MAX_PANES_PER_TAB = 3 as const;
