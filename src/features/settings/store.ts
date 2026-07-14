import { defineStore } from 'pinia';
import { reactive } from 'vue';
import type { CursorStyle, TerminalSettings } from './types';

const STORAGE_KEY = 'nexashell-settings';

const DEFAULT_TERMINAL: TerminalSettings = {
  cursorStyle: 'block',
  cursorBlink: true,
  fontSize: 14,
  fontFamily:
    'ui-monospace, Monaco, Menlo, Consolas, "Cascadia Code", "Ubuntu Mono", monospace',
  scrollback: 80000,
};

function loadTerminal(): TerminalSettings {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored) {
    try {
      const parsed = JSON.parse(stored);
      const terminal = parsed.terminal || parsed;
      if (
        terminal &&
        terminal.fontFamily === 'Monaco, Menlo, Ubuntu Mono, monospace'
      ) {
        terminal.fontFamily = DEFAULT_TERMINAL.fontFamily;
      }
      return { ...DEFAULT_TERMINAL, ...terminal };
    } catch (e) {
      console.error('Failed to parse settings from localStorage', e);
    }
  }
  return { ...DEFAULT_TERMINAL };
}

function persistSettings(terminal: TerminalSettings) {
  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({ terminal })
  );
}

export const useSettingsStore = defineStore('settings', () => {
  const terminal = reactive<TerminalSettings>(loadTerminal());

  function setCursorStyle(style: CursorStyle) {
    terminal.cursorStyle = style;
    persistSettings({ ...terminal });
  }

  function setCursorBlink(blink: boolean) {
    terminal.cursorBlink = blink;
    persistSettings({ ...terminal });
  }

  function setFontSize(size: number) {
    terminal.fontSize = size;
    persistSettings({ ...terminal });
  }

  function saveSettings() {
    persistSettings({ ...terminal });
  }

  return {
    terminal,
    setCursorStyle,
    setCursorBlink,
    setFontSize,
    saveSettings,
  };
});
