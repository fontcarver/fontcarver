import type { DocWarning } from '../api'

/** Translation options supplied for a kernel warning. */
export interface WarningTranslationOptions {
  [key: string]: unknown
  defaultValue: string
  message: string
}

/** Minimal translation function used when rendering a kernel warning. */
export type WarningTranslator = (
  key: string,
  options?: WarningTranslationOptions,
) => string

const WARNING_KEYS: Readonly<Record<string, string>> = {
  'cff-hints-not-imported': 'warning.cffHintsNotImported',
  'glyph-names-synthesized': 'warning.glyphNamesSynthesized',
  'collection-face-opened': 'warning.collectionFaceOpened',
}

/** Translates a known warning code and preserves the kernel detail as fallback. */
export function warningMessage(
  warning: DocWarning,
  t: WarningTranslator,
): string {
  const key = WARNING_KEYS[warning.code]
  return key
    ? t(key, { defaultValue: warning.message, message: warning.message })
    : warning.message
}
