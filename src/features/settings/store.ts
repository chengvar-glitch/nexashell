import { defineStore } from 'pinia';
import type { SettingsState, CursorStyle } from './types';

const STORAGE_KEY = 'nexashell-settings';

const DEFAULT_SETTINGS: SettingsState = {
  terminal: {
    cursorStyle: 'block',
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'ui-monospace, Monaco, Menlo, Consolas, "Cascadia Code", "Ubuntu Mono", monospace',
    scrollback: 80000,
  },
};

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        // Migrate old default font family if necessary
        if (
          parsed.terminal &&
          parsed.terminal.fontFamily === 'Monaco, Menlo, Ubuntu Mono, monospace'
        ) {
          parsed.terminal.fontFamily = DEFAULT_SETTINGS.terminal.fontFamily;
        }
        return {
          ...DEFAULT_SETTINGS,
          ...parsed,
          terminal: { ...DEFAULT_SETTINGS.terminal, ...(parsed.terminal || {}) },
        };
      } catch (e) {
        console.error('Failed to parse settings from localStorage', e);
      }
    }
    return { ...DEFAULT_SETTINGS };
  },

  actions: {
    setCursorStyle(style: CursorStyle) {
      this.terminal.cursorStyle = style;
      this.saveSettings();
    },

    setCursorBlink(blink: boolean) {
      this.terminal.cursorBlink = blink;
      this.saveSettings();
    },

    setFontSize(size: number) {
      this.terminal.fontSize = size;
      this.saveSettings();
    },

    saveSettings() {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.$state));
    },
  },
});
