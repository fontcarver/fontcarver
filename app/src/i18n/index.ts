import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

import en from './en.json'
import zhCN from './zh-CN.json'

/** Language codes the interface ships with. */
export const SUPPORTED_LANGUAGES = ['en', 'zh-CN'] as const

/** One of {@link SUPPORTED_LANGUAGES}. */
export type Language = (typeof SUPPORTED_LANGUAGES)[number]

/** Language used when a stored or requested code is not supported. */
export const DEFAULT_LANGUAGE: Language = 'en'

/** Narrows an arbitrary string to a supported language code. */
export function normaliseLanguage(value: string | null | undefined): Language {
  return SUPPORTED_LANGUAGES.includes(value as Language)
    ? (value as Language)
    : DEFAULT_LANGUAGE
}

if (!i18n.isInitialized) {
  void i18n.use(initReactI18next).init({
    resources: {
      en: { translation: en },
      'zh-CN': { translation: zhCN },
    },
    lng: DEFAULT_LANGUAGE,
    fallbackLng: DEFAULT_LANGUAGE,
    supportedLngs: [...SUPPORTED_LANGUAGES],
    showSupportNotice: false,
    interpolation: {
      escapeValue: false,
    },
  })
}

export { i18n }
