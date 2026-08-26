import { createI18n } from 'vue-i18n';
import en from './locales/en.ts';

type MessageSchema = typeof en;

export const AVAILABLE_LOCALES = ['en', 'zh'] as const;

const storedLocale: string = localStorage.getItem('language') || 'en';
const initialLocale = storedLocale === 'zh' ? 'zh' : 'en';

const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: 'en',
  messages: { en },
});

// Non-default locale files are partial patches on top of the English default;
// vue-i18n's own mergeLocaleMessage does the deep merge.
const loadedMessages: Record<string, MessageSchema> = { en };

async function loadLocale(locale: string): Promise<MessageSchema> {
  if (loadedMessages[locale]) return loadedMessages[locale];
  let mod: Record<string, unknown>;
  switch (locale) {
    case 'zh':
      mod = (await import('./locales/zh.ts')).default as Record<string, unknown>;
      break;
    default:
      return en;
  }
  i18n.global.mergeLocaleMessage(locale, mod);
  loadedMessages[locale] = i18n.global.getLocaleMessage(locale) as MessageSchema;
  return loadedMessages[locale];
}

export { i18n };

/**
 * Typed accessor for the current locale ref — avoids `(i18n as any).global`
 * casts scattered across components.
 */
export function currentLocaleRef(): import('vue').Ref<string> {
  return i18n.global.locale;
}

export async function setLocale(locale: string) {
  if (!AVAILABLE_LOCALES.includes(locale as 'en' | 'zh')) {
    locale = 'en';
  }
  const msg = await loadLocale(locale);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (i18n as any).global.setLocaleMessage(locale, msg);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (i18n.global.locale as any).value = locale;
  localStorage.setItem('language', locale);
}

/**
 * Initialize the active locale (loading the matching message table when it is
 * not the default). Exported so entrypoints can await it during bootstrap.
 */
export async function initLocale(): Promise<void> {
  if (initialLocale !== 'en') {
    await setLocale(initialLocale);
  }
}

// Kick off locale loading at module load. `.catch` prevents an unhandled
// rejection if a dynamic import fails, and entrypoints may additionally await
// initLocale() above without double work (loadLocale is cached).
initLocale().catch(() => {
  // Non-fatal: the default English locale keeps the app usable.
});
