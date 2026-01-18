import { createI18n } from 'vue-i18n';
import en from './locales/en.ts';
import zh from './locales/zh.ts';
import zhTW from './locales/zh-TW.ts';
import ja from './locales/ja.ts';
import ko from './locales/ko.ts';
import fr from './locales/fr.ts';
import de from './locales/de.ts';
import ru from './locales/ru.ts';
import ar from './locales/ar.ts';
import es from './locales/es.ts';
import ms from './locales/ms.ts';
import it from './locales/it.ts';

type MessageSchema = typeof en;

const rawMessages: Record<string, MessageSchema> = {
  en,
  zh: zh as MessageSchema,
  'zh-TW': zhTW as MessageSchema,
  ja: ja as MessageSchema,
  ko: ko as MessageSchema,
  fr: fr as MessageSchema,
  de: de as MessageSchema,
  ru: ru as MessageSchema,
  ar: ar as MessageSchema,
  es: es as MessageSchema,
  ms: ms as MessageSchema,
  it: it as MessageSchema,
};

function mergeDefaults(base: unknown, target: unknown): unknown {
  if (target === undefined || target === null) {
    return base;
  }
  if (Array.isArray(base)) {
    return target !== undefined ? target : base;
  }
  if (typeof base === 'object' && base !== null) {
    const res: Record<string, unknown> = {};
    const baseObj = base as Record<string, unknown>;
    const targetObj =
      typeof target === 'object' && target !== null
        ? (target as Record<string, unknown>)
        : {};

    for (const key of Object.keys(baseObj)) {
      const baseVal = baseObj[key];
      const targetVal = Object.prototype.hasOwnProperty.call(targetObj, key)
        ? targetObj[key]
        : undefined;
      if (typeof baseVal === 'object' && baseVal !== null) {
        res[key] = mergeDefaults(baseVal, targetVal);
      } else {
        res[key] = targetVal !== undefined ? targetVal : baseVal;
      }
    }
    // include any extra keys present in target
    for (const key of Object.keys(targetObj)) {
      if (!Object.prototype.hasOwnProperty.call(res, key)) {
        res[key] = targetObj[key];
      }
    }
    return res;
  }
  return target !== undefined ? target : base;
}

const messages: Record<string, MessageSchema> = {};
for (const key of Object.keys(rawMessages)) {
  messages[key] = (
    key === 'en'
      ? rawMessages.en
      : mergeDefaults(rawMessages.en, rawMessages[key])
  ) as MessageSchema;
}

const availableLocales = Object.keys(messages);
const storedLocale = localStorage.getItem('language');
const locale =
  storedLocale && availableLocales.includes(storedLocale) ? storedLocale : 'en';

export const i18n = createI18n({
  legacy: false,
  locale,
  fallbackLocale: 'en',
  messages,
});
