import { createI18n } from 'vue-i18n';
import en from './locales/en.ts';

type MessageSchema = typeof en;

export const AVAILABLE_LOCALES = ['en', 'zh', 'zh-TW', 'ja', 'ko', 'fr', 'de', 'ru', 'ar', 'es', 'ms', 'it'];

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
    case 'zh': mod = await import('./locales/zh.ts'); break;
    case 'zh-TW': mod = await import('./locales/zh-TW.ts'); break;
    case 'ja': mod = await import('./locales/ja.ts'); break;
    case 'ko': mod = await import('./locales/ko.ts'); break;
    case 'fr': mod = await import('./locales/fr.ts'); break;
    case 'de': mod = await import('./locales/de.ts'); break;
    case 'ru': mod = await import('./locales/ru.ts'); break;
    case 'ar': mod = await import('./locales/ar.ts'); break;
    case 'es': mod = await import('./locales/es.ts'); break;
    case 'ms': mod = await import('./locales/ms.ts'); break;
    case 'it': mod = await import('./locales/it.ts'); break;
    default: return en;
  }
  const merged = mergeDefaults(en, mod) as MessageSchema;
  loadedMessages[locale] = merged;
  return merged;
}

const storedLocale: string = localStorage.getItem('language') || 'en';
const locale = storedLocale === 'en' || AVAILABLE_LOCALES.includes(storedLocale) ? storedLocale : 'en';

export const i18n = createI18n({
  legacy: false,
  locale,
  fallbackLocale: 'en',
  messages: { en },
});

export async function setLocale(locale: string) {
  const msg = await loadLocale(locale);
  i18n.global.setLocaleMessage(locale, msg);
  i18n.global.locale.value = locale as any;
  localStorage.setItem('language', locale);
}

if (locale !== 'en') {
  loadLocale(locale);
}
