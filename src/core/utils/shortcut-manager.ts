import {
  quitApp,
  closeWindow,
  createNewLocalTab,
  focusSearch,
} from '@/core/utils/app-utils';

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
  private handleKeyDown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    const isInputElement =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.contentEditable === 'true';

    // Global shortcuts that should trigger even in input fields
    const isGlobalShortcut =
      (['p', 'w', 't', 'q', ','].includes(event.key.toLowerCase()) &&
        (event.metaKey || event.ctrlKey)) ||
      event.key === 'Escape';

    if (isInputElement) {
      if (isGlobalShortcut) {
        // Allow global shortcuts to proceed
      } else if (event.key === 'Tab') {
        // Special handling: allow Tab key in search-related areas
        const isInSearchBox = target.closest('.search-container') !== null;
        const isInSearchDropdown = target.closest('.search-dropdown') !== null;
        const searchDropdownVisible =
          document.querySelector('.search-dropdown');

        if (!(searchDropdownVisible && (isInSearchBox || isInSearchDropdown))) {
          event.preventDefault();
          return;
        }
      } else {
        // Don't trigger other shortcuts in input fields
        return;
      }
    }

    if (event.key === 'Tab') {
      const isInSearchBox = target.closest('.search-container') !== null;
      const isInSearchDropdown = target.closest('.search-dropdown') !== null;
      const searchDropdownVisible = document.querySelector('.search-dropdown');

      if (!(searchDropdownVisible && (isInSearchBox || isInSearchDropdown))) {
        event.preventDefault();
        return;
      }
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
  CLOSE_WINDOW: {
    key: 'w',
    metaKey: IS_MAC,
    ctrlKey: !IS_MAC,
    shiftKey: false,
    altKey: false,
    description: 'Close window',
    handler: async () => {
      await closeWindow();
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
      window.dispatchEvent(new CustomEvent('app:open-settings'));
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
      window.dispatchEvent(new CustomEvent('app:open-ssh-form'));
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
      window.dispatchEvent(new CustomEvent('app:close-dialog'));
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
      window.dispatchEvent(new CustomEvent('app:close-tab'));
    },
  },
};
