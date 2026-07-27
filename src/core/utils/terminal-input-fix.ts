import type { Terminal } from '@xterm/xterm';
import { isMacOSBrowser } from '@/core/utils/platform/platform-detection';

type TerminalWithInput = Pick<Terminal, 'input' | 'textarea'>;

const MAX_SYMBOL_LEN = 8;
const SYMBOL_RE = /^[\p{P}\p{S}]+$/u;
const HANDLED_INPUT_TYPES = new Set([
  'insertText',
  'insertCompositionText',
]);

function isMacWebKit(): boolean {
  if (typeof navigator === 'undefined') return false;
  return isMacOSBrowser() && navigator.userAgent.includes('AppleWebKit');
}

function getPrintableSymbol(data: string | null | undefined): string | null {
  if (!data || data.length === 0 || data.length > MAX_SYMBOL_LEN) return null;
  return SYMBOL_RE.test(data) ? data : null;
}

/**
 * Bridge macOS WebKit's `beforeinput` symbol delivery into xterm.js.
 *
 * On macOS WKWebView with a Chinese IME, punctuation shortcuts like Shift+/
 * (→ '？'), Shift+4 (→ '¥'), etc. are delivered via the helper textarea's
 * `beforeinput` event instead of the normal keydown → input pipeline that
 * xterm.js listens to. Without this bridge, the first press is swallowed
 * and the user must press the key twice to get one character through.
 *
 * Adapted from the fix landed in hanshuaikang/nezha #97.
 *
 * @returns disposer to remove the listeners.
 */
export function attachMacWebKitIMESymbolFix(
  term: TerminalWithInput
): () => void {
  if (!isMacWebKit() || !term.textarea) return () => {};

  const textarea = term.textarea;
  let keydownHandledByXterm: string | null = null;

  const handleKeyDown = (event: KeyboardEvent) => {
    keydownHandledByXterm = null;
    const isPlainShiftSymbol =
      event.keyCode !== 229 &&
      event.shiftKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      getPrintableSymbol(event.key) !== null;
    if (isPlainShiftSymbol) {
      keydownHandledByXterm = event.key;
    }
  };

  const handleBeforeInput = (event: InputEvent) => {
    const symbol = getPrintableSymbol(event.data);
    if (!HANDLED_INPUT_TYPES.has(event.inputType) || symbol === null) return;
    if (keydownHandledByXterm === symbol) {
      keydownHandledByXterm = null;
      return;
    }
    term.input(symbol);
    event.preventDefault();
  };

  textarea.addEventListener('keydown', handleKeyDown);
  textarea.addEventListener('beforeinput', handleBeforeInput);

  return () => {
    textarea.removeEventListener('keydown', handleKeyDown);
    textarea.removeEventListener('beforeinput', handleBeforeInput);
  };
}
