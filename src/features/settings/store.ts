import { defineStore } from 'pinia';
import { reactive, readonly } from 'vue';
import { createLogger } from '@/core/utils/logger';
import type { CursorStyle, TerminalSettings } from './types';
import { TERMINAL_THEME_KEYS } from '@/core/terminal-themes';
import type { TerminalThemeKey } from '@/core/terminal-themes';

const logger = createLogger('SETTINGS_STORE');

const STORAGE_KEY = 'nexashell-settings';

// Clamp bounds for numeric terminal settings.
const FONT_SIZE_MIN = 6;
const FONT_SIZE_MAX = 72;
const SCROLLBACK_MIN = 1000;
const SCROLLBACK_MAX = 1000000;

const DEFAULT_TERMINAL: TerminalSettings = {
  cursorStyle: 'block',
  cursorBlink: true,
  fontSize: 14,
  fontFamily:
    'ui-monospace, Monaco, Menlo, Consolas, "Cascadia Code", "Ubuntu Mono", monospace',
  scrollback: 80000,
  theme: 'system',
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
      // Validate the theme key; fall back to system if absent or invalid.
      if (
        !terminal.theme ||
        !TERMINAL_THEME_KEYS.includes(terminal.theme as TerminalThemeKey)
      ) {
        terminal.theme = 'system';
      }
      return { ...DEFAULT_TERMINAL, ...terminal };
    } catch (e) {
      logger.error('Failed to parse settings from localStorage', e);
    }
  }
  return { ...DEFAULT_TERMINAL };
}

function persistSettings(terminal: TerminalSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ terminal }));
  } catch (e) {
    logger.error('Failed to persist settings', e);
  }
}

export const useSettingsStore = defineStore('settings', () => {
  const terminal = reactive<TerminalSettings>(loadTerminal());

  function setCursorStyle(style: CursorStyle) {
    // Ignore invalid cursor styles rather than persisting garbage.
    if (style !== 'block' && style !== 'underline' && style !== 'bar') {
      logger.warn('Ignoring invalid cursor style', { style });
      return;
    }
    terminal.cursorStyle = style;
    persistSettings({ ...terminal });
  }

  function setCursorBlink(blink: boolean) {
    terminal.cursorBlink = !!blink;
    persistSettings({ ...terminal });
  }

  function setFontSize(size: number) {
    // Zero / NaN / negative sizes would break the terminal; clamp instead of
    // persisting an invalid value.
    const clamped = Number.isFinite(size)
      ? Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(size)))
      : DEFAULT_TERMINAL.fontSize;
    terminal.fontSize = clamped;
    persistSettings({ ...terminal });
  }

  function setTheme(theme: TerminalThemeKey) {
    // Reject unknown theme keys so a bad value can't corrupt rendering.
    if (!TERMINAL_THEME_KEYS.includes(theme)) {
      logger.warn('Ignoring invalid terminal theme', { theme });
      return;
    }
    terminal.theme = theme;
    persistSettings({ ...terminal });
  }

  function setScrollback(scrollback: number) {
    const clamped = Number.isFinite(scrollback)
      ? Math.min(SCROLLBACK_MAX, Math.max(SCROLLBACK_MIN, Math.round(scrollback)))
      : DEFAULT_TERMINAL.scrollback;
    terminal.scrollback = clamped;
    persistSettings({ ...terminal });
  }

  return {
    // readonly: consumers must go through the setters, which persist and
    // validate, so a reactive leak cannot silently bypass persistence.
    terminal: readonly(terminal),
    setCursorStyle,
    setCursorBlink,
    setFontSize,
    setTheme,
    setScrollback,
  };
});
