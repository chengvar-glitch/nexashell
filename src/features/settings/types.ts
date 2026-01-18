export type CursorStyle = 'block' | 'underline' | 'bar';

export interface TerminalSettings {
  cursorStyle: CursorStyle;
  cursorBlink: boolean;
  fontSize: number;
  fontFamily: string;
  scrollback: number;
}

export interface SettingsState {
  terminal: TerminalSettings;
}
