import {
  quitApp,
} from '@/features/window';
import { createNewLocalTab } from '@/features/tabs';
import { eventBus } from './event-bus';
import { APP_EVENTS } from '@/core/constants';

const focusSearch = () => eventBus.emit(APP_EVENTS.FOCUS_SEARCH);
const splitVertical = () => eventBus.emit(APP_EVENTS.SPLIT_VERTICAL);
const splitHorizontal = () => eventBus.emit(APP_EVENTS.SPLIT_HORIZONTAL);

export interface ShortcutConfig {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  description: string;
  handler: () => void;
}

export class ShortcutManager {
  private shortcuts: Map<string, ShortcutConfig> = new Map();
  private listenerBound: boolean = false;

  constructor() {
    this.handleKeyDown = this.handleKeyDown.bind(this);
  }

  /**
   * Register a keyboard shortcut
   * @param config Shortcut configuration
   */
  register(config: ShortcutConfig) {
    const key = this.generateKey(config);
    this.shortcuts.set(key, config);

    if (!this.listenerBound) {
      this.bindListener();
    }
  }

  /**
   * Unregister a keyboard shortcut
   * @param config Shortcut configuration
   */
  unregister(config: Omit<ShortcutConfig, 'handler'>) {
    const key = this.generateKey(config);
    this.shortcuts.delete(key);
  }

  /**
   * Unregister all keyboard shortcuts
   */
  unregisterAll() {
    this.shortcuts.clear();
    this.unbindListener();
  }

  /**
   * Generate unique identifier for shortcut
   */
  private generateKey(config: Partial<ShortcutConfig>): string {
    const key = config.key?.toLowerCase();
    return `${key}_${!!config.ctrlKey}_${!!config.metaKey}_${!!config.shiftKey}_${!!config.altKey}`;
  }

  /**
   * Bind keyboard event listener
   */
  private bindListener() {
    window.addEventListener('keydown', this.handleKeyDown, { passive: false });
    this.listenerBound = true;
  }

  /**
   * Unbind keyboard event listener
   */
  private unbindListener() {
    window.removeEventListener('keydown', this.handleKeyDown);
    this.listenerBound = false;
  }

  /**
   * Handle keyboard events
   */
  /** True when the key event originates inside a terminal (xterm.js) instance. */
  private isInTerminal(event: KeyboardEvent): boolean {
    const target = event.target as HTMLElement | null;
    if (!target || typeof target.closest !== 'function') return false;
    // xterm renders a real <textarea>; a global shortcut must not swallow
    // keys that belong to the terminal (Ctrl+D EOF, Ctrl+Q, Cmd+... etc.).
    return target.closest('.xterm') !== null;
  }

  /** True when the event should trigger a global shortcut even in an input. */
  private isGlobalShortcut(key: string, event: KeyboardEvent): boolean {
    return (
      (['p', 'w', 't', 'q', ',', 'd'].includes(key.toLowerCase()) &&
        (event.metaKey || event.ctrlKey)) ||
      event.key === 'Escape'
    );
  }

  /** True when Tab should open/focus the already-visible search dropdown. */
  private isSearchDropdownTabAction(event: KeyboardEvent): boolean {
    const target = event.target as HTMLElement | null;
    if (!target || typeof target.closest !== 'function') return false;
    const searchDropdownVisible = !!document.querySelector('.search-dropdown');
    return (
      searchDropdownVisible &&
      (target.closest('.search-container') !== null ||
        target.closest('.search-dropdown') !== null)
    );
  }

  private handleKeyDown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    const isInputElement =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.contentEditable === 'true';

    // When the event targets a terminal, let xterm keep control of terminal
    // input. On macOS the shell never sees app-level Cmd+<key> combos, and
    // those are the app's shortcuts (Cmd+, settings, Cmd+T new tab, Cmd+W
    // close, Cmd+D split, Cmd+P search), so let metaKey events through.
    // Block everything else (Ctrl+D/EOF, Ctrl+Q, plain keys) so the terminal
    // owns them instead of a global shortcut swallowing them.
    if (this.isInTerminal(event) && !event.metaKey) {
      return;
    }

    if (isInputElement) {
      if (this.isGlobalShortcut(event.key, event)) {
        // Allow global shortcuts to proceed
      } else if (event.key === 'Tab') {
        // Only intercept Tab when it performs an actual action (focusing the
        // visible search dropdown); otherwise let native focus navigation work.
        if (this.isSearchDropdownTabAction(event)) {
          // fall through to shortcut resolution
        } else {
          return;
        }
      } else {
        // Don't trigger other shortcuts in input fields
        return;
      }
    }

    if (event.key === 'Tab' && !this.isSearchDropdownTabAction(event)) {
      // Let native Tab navigation proceed instead of blocking it globally.
      return;
    }

    const key = this.generateKey({
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
    });

    const shortcut = this.shortcuts.get(key);
    if (shortcut) {
      event.preventDefault();
      shortcut.handler();
    }
  }

  /**
   * Get all registered shortcuts
   */
  getAllShortcuts(): ShortcutConfig[] {
    return Array.from(this.shortcuts.values());
  }

  /**
   * Format shortcut display text
   */
  static formatShortcut(config: Partial<ShortcutConfig>): string {
    const keys: string[] = [];

    if (config.ctrlKey) keys.push('Ctrl');
    if (config.metaKey)
      keys.push(navigator.userAgent.includes('Mac') ? 'Cmd' : 'Ctrl');
    if (config.shiftKey) keys.push('Shift');
    if (config.altKey) keys.push('Alt');

    if (config.key) {
      let key = config.key;
      if (key === ',') key = ',';
      if (key === ' ') key = 'Space';
      if (key === 'Escape') key = 'Esc';
      keys.push(key.toUpperCase());
    }

    return keys.join('+');
  }
}

export const shortcutManager = new ShortcutManager();

const IS_MAC =
  typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac');

export const PredefinedShortcuts = {
  QUIT_APP: {
    key: 'q',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'Quit application',
    handler: async () => {
      await quitApp();
    },
  },
  OPEN_SETTINGS: {
    key: ',',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'Open settings',
    handler: () => {
      window.dispatchEvent(new CustomEvent(APP_EVENTS.OPEN_SETTINGS));
    },
  },
  NEW_LOCAL_TAB: {
    key: 't',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: true,
    altKey: false,
    description: 'New local terminal tab',
    handler: () => {
      createNewLocalTab();
    },
  },
  NEW_SSH_TAB: {
    key: 't',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'New SSH connection tab',
    handler: () => {
      window.dispatchEvent(new CustomEvent(APP_EVENTS.OPEN_SSH_FORM));
    },
  },
  FOCUS_SEARCH: {
    key: 'p',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'Focus search box',
    handler: () => {
      focusSearch();
    },
  },
  CLOSE_DIALOG: {
    key: 'Escape',
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    description: 'Close dialog',
    handler: () => {
      window.dispatchEvent(new CustomEvent(APP_EVENTS.CLOSE_DIALOG));
    },
  },
  CLOSE_CURRENT_TAB: {
    key: 'w',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'Close current tab',
    handler: () => {
      window.dispatchEvent(new CustomEvent(APP_EVENTS.CLOSE_TAB));
    },
  },
  SPLIT_VERTICAL: {
    key: 'd',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'Split pane vertically',
    handler: splitVertical,
  },
  SPLIT_HORIZONTAL: {
    key: 'd',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: true,
    altKey: false,
    description: 'Split pane horizontally',
    handler: splitHorizontal,
  },
  COMMAND_PALETTE: {
    key: 'p',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: true,
    altKey: false,
    description: 'Open command palette',
    handler: () => {
      eventBus.emit(APP_EVENTS.COMMAND_PALETTE);
    },
  },
};
