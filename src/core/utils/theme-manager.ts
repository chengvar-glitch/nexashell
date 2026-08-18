export type ThemeMode = 'auto' | 'light' | 'dark';

export type AccentKey = 'blue' | 'graphite' | 'purple' | 'green' | 'orange';

const THEME_STORAGE_KEY = 'nexashell-theme';
const ACCENT_STORAGE_KEY = 'nexashell-accent';

/**
 * Event dispatched whenever the theme mode or accent changes. Kept as a local
 * constant (it is not part of the shared APP_EVENTS set) so both setters can
 * emit a consistent payload shape.
 */
export const THEME_CHANGED_EVENT = 'theme-changed';

const ACCENT_KEYS: AccentKey[] = [
  'blue',
  'graphite',
  'purple',
  'green',
  'orange',
];

class ThemeManager {
  private currentTheme: ThemeMode = 'auto';
  private currentAccent: AccentKey = 'blue';
  private mediaQuery: MediaQueryList | null = null;

  constructor() {
    this.loadTheme();
    this.loadAccent();
    this.initMediaQuery();
  }

  private initMediaQuery() {
    if (typeof window !== 'undefined' && window.matchMedia) {
      this.mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      this.mediaQuery.addEventListener('change', () => {
        if (this.currentTheme === 'auto') {
          this.applyTheme('auto');
        }
      });
    }
  }

  private loadTheme() {
    try {
      const saved = localStorage.getItem(THEME_STORAGE_KEY);
      if (saved && ['auto', 'light', 'dark'].includes(saved)) {
        this.currentTheme = saved as ThemeMode;
      }
    } catch {
      // Errors are non-fatal: fall back to defaults silently
    }
  }

  private saveTheme(theme: ThemeMode) {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Errors are non-fatal: theme just won't persist
    }
  }

  private loadAccent() {
    try {
      const saved = localStorage.getItem(ACCENT_STORAGE_KEY);
      if (saved && ACCENT_KEYS.includes(saved as AccentKey)) {
        this.currentAccent = saved as AccentKey;
      }
    } catch {
      // Errors are non-fatal: fall back to the default accent silently
    }
  }

  private saveAccent(accent: AccentKey) {
    try {
      localStorage.setItem(ACCENT_STORAGE_KEY, accent);
    } catch {
      // Errors are non-fatal: accent just won't persist
    }
  }

  private applyAccent(accent: AccentKey) {
    const root = document.documentElement;

    // The default 'blue' accent is baseline; clear the attribute so it is
    // not special-cased and CSS only needs overrides for non-blue accents.
    if (accent === 'blue') {
      delete root.dataset.accent;
    } else {
      root.dataset.accent = accent;
    }
  }

  private getSystemTheme(): 'light' | 'dark' {
    if (this.mediaQuery) {
      return this.mediaQuery.matches ? 'dark' : 'light';
    }
    return 'light';
  }

  private applyTheme(theme: ThemeMode) {
    const root = document.documentElement;

    // Remove existing theme classes
    root.classList.remove('theme-light', 'theme-dark');

    // Determine actual theme to apply
    let actualTheme: 'light' | 'dark';
    if (theme === 'auto') {
      actualTheme = this.getSystemTheme();
    } else {
      actualTheme = theme;
    }

    // Apply theme class
    root.classList.add(`theme-${actualTheme}`);

    // Set color-scheme for native elements
    root.style.colorScheme = actualTheme;
  }

  setTheme(theme: ThemeMode) {
    this.currentTheme = theme;
    this.saveTheme(theme);
    this.applyTheme(theme);

    // Dispatch event for other components, with a consistent payload shape
    // ({ theme, accent }) matching setAccent below.
    window.dispatchEvent(
      new CustomEvent(THEME_CHANGED_EVENT, {
        detail: { theme, accent: this.currentAccent },
      })
    );
  }

  getTheme(): ThemeMode {
    return this.currentTheme;
  }

  setAccent(accent: AccentKey) {
    this.currentAccent = accent;
    this.saveAccent(accent);
    this.applyAccent(accent);

    // Dispatch event for other components, keeping theme-changed working for
    // both mode and accent changes with a consistent { theme, accent } shape.
    window.dispatchEvent(
      new CustomEvent(THEME_CHANGED_EVENT, {
        detail: { theme: this.currentTheme, accent },
      })
    );
  }

  getAccent(): AccentKey {
    return this.currentAccent;
  }

  getActualTheme(): 'light' | 'dark' {
    if (this.currentTheme === 'auto') {
      return this.getSystemTheme();
    }
    return this.currentTheme;
  }

  initialize() {
    this.applyTheme(this.currentTheme);
    this.applyAccent(this.currentAccent);
  }
}

export const themeManager = new ThemeManager();
