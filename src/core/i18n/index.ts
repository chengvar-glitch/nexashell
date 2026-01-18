import { createI18n } from 'vue-i18n';
import en from './locales/en';
import zh from './locales/zh';
import zhTW from './locales/zh-TW';
import ja from './locales/ja';
import ko from './locales/ko';
import fr from './locales/fr';
import de from './locales/de';
import ru from './locales/ru';
import ar from './locales/ar';
import es from './locales/es';
import ms from './locales/ms';
import it from './locales/it';

const rawMessages: Record<string, any> = {
  en,
  zh,
  'zh-TW': zhTW,
  ja,
  ko,
  fr,
  de,
  ru,
  ar,
  es,
  ms,
  it,
};

function mergeDefaults(base: any, target: any): any {
  if (target === undefined || target === null) {
    return base;
  }
  if (Array.isArray(base)) {
    return target !== undefined ? target : base;
  }
  if (typeof base === 'object' && base !== null) {
    const res: any = {};
    for (const key of Object.keys(base)) {
      const baseVal = base[key];
      const targetVal = Object.prototype.hasOwnProperty.call(target, key) ? target[key] : undefined;
      if (typeof baseVal === 'object' && baseVal !== null) {
        res[key] = mergeDefaults(baseVal, targetVal);
      } else {
        res[key] = targetVal !== undefined ? targetVal : baseVal;
      }
    }
    // include any extra keys present in target
    if (typeof target === 'object' && target !== null) {
      for (const key of Object.keys(target)) {
        if (!Object.prototype.hasOwnProperty.call(res, key)) {
          res[key] = target[key];
        }
      }
    }
    return res;
  }
  return target !== undefined ? target : base;
}

const messages: Record<string, any> = {};
for (const key of Object.keys(rawMessages)) {
  messages[key] = key === 'en' ? rawMessages.en : mergeDefaults(rawMessages.en, rawMessages[key]);
}

const availableLocales = Object.keys(messages);
const storedLocale = localStorage.getItem('language');
const locale = storedLocale && availableLocales.includes(storedLocale) ? storedLocale : 'en';

export const i18n = createI18n({
  legacy: false,
  locale,
  fallbackLocale: 'en',
  messages,
});
