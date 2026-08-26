/**
 * Terminal color themes for the xterm.js terminal.
 *
 * Each theme is a full xterm `ITheme`-compatible object (surface colors) plus
 * the 16-color ANSI palette. `'system'` means: follow the app's light/dark
 * mode (`themeManager.getActualTheme()`) and pick the matching preset.
 *
 * Colors are the well-known community presets (One Dark, Solarized, iTerm2
 * defaults, etc.) so they match what users know from iTerm2/Tabby.
 */

export type TerminalThemeKey =
  | 'system'
  | 'oneark'
  | 'modernDark'
  | 'modernLight'
  | 'solarizedDark'
  | 'solarizedLight'
  | 'githubDark'
  | 'githubLight';

export interface TerminalTheme {
  /** xterm ITheme surface colors. */
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  selectionForeground: string;
  /** 16-color ANSI palette (index 0..15). */
  ansi: [string, string, string, string, string, string, string, string, string, string, string, string, string, string, string, string];
}

/** The "dark" and "light" preset used by the `system` option. */
const TERMINAL_PRESETS: Record<Exclude<TerminalThemeKey, 'system'>, TerminalTheme> = {
  oneark: {
    background: '#282c34',
    foreground: '#abb2bf',
    cursor: '#528bff',
    cursorAccent: '#282c34',
    selectionBackground: '#3e4451',
    selectionForeground: '#abb2bf',
    ansi: [
      '#282c34', // black
      '#e06c75', // red
      '#98c379', // green
      '#e5c07b', // yellow
      '#61afef', // blue
      '#c678dd', // magenta
      '#56b6c2', // cyan
      '#abb2bf', // white
      '#5c6370', // bright black
      '#e06c75', // bright red
      '#98c379', // bright green
      '#e5c07b', // bright yellow
      '#61afef', // bright blue
      '#c678dd', // bright magenta
      '#56b6c2', // bright cyan
      '#ffffff', // bright white
    ],
  },
  modernDark: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    cursorAccent: '#1e1e1e',
    selectionBackground: '#facc15',
    selectionForeground: '#000000',
    ansi: [
      '#000000',
      '#cd3131',
      '#0dbc79',
      '#e5e510',
      '#2472c8',
      '#bc3fbc',
      '#11a8cd',
      '#e5e5e5',
      '#666666',
      '#f14c4c',
      '#23d18b',
      '#f5f543',
      '#3b8eea',
      '#d670d6',
      '#29b8db',
      '#ffffff',
    ],
  },
  modernLight: {
    background: '#ffffff',
    foreground: '#333333',
    cursor: '#0066cc',
    cursorAccent: '#ffffff',
    selectionBackground: '#b5d5ff',
    selectionForeground: '#000000',
    ansi: [
      '#000000',
      '#cd3131',
      '#0dbc79',
      '#949800',
      '#0451a5',
      '#bc05bc',
      '#0598bc',
      '#555555',
      '#666666',
      '#cd3131',
      '#14ce14',
      '#b5ba00',
      '#0451a5',
      '#bc05bc',
      '#0598bc',
      '#a5a5a5',
    ],
  },
  solarizedDark: {
    background: '#002b36',
    foreground: '#839496',
    cursor: '#839496',
    cursorAccent: '#002b36',
    selectionBackground: '#073642',
    selectionForeground: '#839496',
    ansi: [
      '#073642',
      '#dc322f',
      '#859900',
      '#b58900',
      '#268bd2',
      '#d33682',
      '#2aa198',
      '#eee8d5',
      '#002b36',
      '#cb4b16',
      '#586e75',
      '#657b83',
      '#839496',
      '#6c71c4',
      '#93a1a1',
      '#fdf6e3',
    ],
  },
  solarizedLight: {
    background: '#fdf6e3',
    foreground: '#657b83',
    cursor: '#657b83',
    cursorAccent: '#fdf6e3',
    selectionBackground: '#eee8d5',
    selectionForeground: '#073642',
    ansi: [
      '#073642',
      '#dc322f',
      '#859900',
      '#b58900',
      '#268bd2',
      '#d33682',
      '#2aa198',
      '#eee8d5',
      '#002b36',
      '#cb4b16',
      '#586e75',
      '#657b83',
      '#839496',
      '#6c71c4',
      '#93a1a1',
      '#fdf6e3',
    ],
  },
  githubDark: {
    background: '#0d1117',
    foreground: '#c9d1d9',
    cursor: '#58a6ff',
    cursorAccent: '#0d1117',
    selectionBackground: '#264f78',
    selectionForeground: '#c9d1d9',
    ansi: [
      '#484f58',
      '#ff7b72',
      '#3fb950',
      '#d29922',
      '#58a6ff',
      '#bc8cff',
      '#39c5cf',
      '#b1bac4',
      '#6e7681',
      '#ffa198',
      '#56d364',
      '#e3b341',
      '#79c0ff',
      '#d2a8ff',
      '#56d4dd',
      '#f0f6fc',
    ],
  },
  githubLight: {
    background: '#ffffff',
    foreground: '#24292f',
    cursor: '#044289',
    cursorAccent: '#ffffff',
    selectionBackground: '#add6ff',
    selectionForeground: '#24292f',
    ansi: [
      '#24292e',
      '#d73a49',
      '#22863a',
      '#b08800',
      '#005cc5',
      '#6f42c1',
      '#3192aa',
      '#6a737d',
      '#959da5',
      '#cb2431',
      '#22863a',
      '#b08800',
      '#005cc5',
      '#6f42c1',
      '#3192aa',
      '#f6f8fa',
    ],
  },
};

/** The preset used by `system` — `'oneark'` (dark) / `'modernLight'` (light). */
const SYSTEM_DARK = 'oneark' as const;
const SYSTEM_LIGHT = 'modernLight' as const;

/**
 * Resolve a `TerminalThemeKey` (plus the app's actual light/dark mode) into a
 * concrete `TerminalTheme`. `'system'` follows the app's current mode.
 */
export function resolveTerminalTheme(
  key: TerminalThemeKey,
  actualMode: 'light' | 'dark'
): TerminalTheme {
  if (key === 'system') {
    return TERMINAL_PRESETS[actualMode === 'dark' ? SYSTEM_DARK : SYSTEM_LIGHT];
  }
  return TERMINAL_PRESETS[key];
}

/** Stable ordered list for UI dropdowns. */
export const TERMINAL_THEME_KEYS: TerminalThemeKey[] = [
  'system',
  'oneark',
  'modernDark',
  'modernLight',
  'solarizedDark',
  'solarizedLight',
  'githubDark',
  'githubLight',
];

const ANSI_COLOR_NAMES = [
  'black',
  'red',
  'green',
  'yellow',
  'blue',
  'magenta',
  'cyan',
  'white',
  'brightBlack',
  'brightRed',
  'brightGreen',
  'brightYellow',
  'brightBlue',
  'brightMagenta',
  'brightCyan',
  'brightWhite',
] as const;

/**
 * Convert a `TerminalTheme` into xterm.js's `ITheme` shape (named ANSI colors,
 * not an index array), which is what `xterm.options.theme` / `setTheme`
 * expects.
 */
export function toXtermTheme(
  theme: TerminalTheme
): Record<
  | 'background'
  | 'foreground'
  | 'cursor'
  | 'cursorAccent'
  | 'selectionBackground'
  | 'selectionForeground',
  string
> &
  Record<(typeof ANSI_COLOR_NAMES)[number], string> {
  const out: Record<string, string> = {
    background: theme.background,
    foreground: theme.foreground,
    cursor: theme.cursor,
    cursorAccent: theme.cursorAccent,
    selectionBackground: theme.selectionBackground,
    selectionForeground: theme.selectionForeground,
  };
  for (let i = 0; i < 16; i++) {
    out[ANSI_COLOR_NAMES[i]] = theme.ansi[i];
  }
  return out as ReturnType<typeof toXtermTheme>;
}
