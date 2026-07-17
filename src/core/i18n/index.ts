import { createI18n } from 'vue-i18n';
import en from './locales/en.ts';

type MessageSchema = typeof en;

export const AVAILABLE_LOCALES = ['en', 'zh'] as const;

function mergeDefaults(base: unknown, target: unknown): unknown {
  if (target === undefined || target === null) return base;
  if (Array.isArray(base)) return target !== undefined ? target : base;
  if (typeof base === 'object' && base !== null) {
    const res: Record<string, unknown> = {};
    const baseObj = base as Record<string, unknown>;
    const targetObj = typeof target === 'object' && target !== null
      ? (target as Record<string, unknown>)
      : {};
    for (const key of Object.keys(baseObj)) {
      const baseVal = baseObj[key];
      const targetVal = Object.prototype.hasOwnProperty.call(targetObj, key) ? targetObj[key] : undefined;
      if (typeof baseVal === 'object' && baseVal !== null) {
        res[key] = mergeDefaults(baseVal, targetVal);
      } else {
        res[key] = targetVal !== undefined ? targetVal : baseVal;
      }
    }
    for (const key of Object.keys(targetObj)) {
      if (!Object.prototype.hasOwnProperty.call(res, key)) res[key] = targetObj[key];
    }
    return res;
  }
  return target !== undefined ? target : base;
}

const loadedMessages: Record<string, MessageSchema> = { en };

async function loadLocale(locale: string): Promise<MessageSchema> {
  if (loadedMessages[locale]) return loadedMessages[locale];
  let mod: Record<string, unknown>;
  switch (locale) {
    case 'zh': mod = (await import('./locales/zh.ts')).default as Record<string, unknown>; break;
    default: return en;
  }
  const merged = mergeDefaults(en, mod) as MessageSchema;
  loadedMessages[locale] = merged;
  return merged;
}

const storedLocale: string = localStorage.getItem('language') || 'en';
const initialLocale = storedLocale === 'zh' ? 'zh' : 'en';

const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: 'en',
  messages: { en },
});

export { i18n };

export async function setLocale(locale: string) {
  if (!AVAILABLE_LOCALES.includes(locale as 'en' | 'zh')) {
    locale = 'en';
  }
  const msg = await loadLocale(locale);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (i18n as any).global.setLocaleMessage(locale, msg);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  i18n.global.locale.value = locale as any;
  localStorage.setItem('language', locale);
}

async function initLocale() {
  if (initialLocale !== 'en') {
    await setLocale(initialLocale);
  }
}

initLocale();
