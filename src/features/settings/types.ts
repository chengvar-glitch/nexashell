import type { TerminalThemeKey } from '@/core/terminal-themes';

export type CursorStyle = 'block' | 'underline' | 'bar';

export interface TerminalSettings {
  cursorStyle: CursorStyle;
  cursorBlink: boolean;
  fontSize: number;
  fontFamily: string;
  scrollback: number;
  /** Terminal color theme key. */
  theme: TerminalThemeKey;
}
