export type ThemeMode = 'auto' | 'light' | 'dark';

const THEME_STORAGE_KEY = 'nexashell-theme';

class ThemeManager {
  private currentTheme: ThemeMode = 'auto';
  private mediaQuery: MediaQueryList | null = null;

  constructor() {
    this.loadTheme();
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
    } catch (error) {
      console.error('Failed to load theme:', error);
    }
  }

  private saveTheme(theme: ThemeMode) {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch (error) {
      console.error('Failed to save theme:', error);
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

    // Dispatch event for other components
    window.dispatchEvent(
      new CustomEvent('theme-changed', {
        detail: { theme },
      })
    );
  }

  getTheme(): ThemeMode {
    return this.currentTheme;
  }

  getActualTheme(): 'light' | 'dark' {
    if (this.currentTheme === 'auto') {
      return this.getSystemTheme();
    }
    return this.currentTheme;
  }

  initialize() {
    this.applyTheme(this.currentTheme);
  }
}

export const themeManager = new ThemeManager();
