import { ApiCallError } from '../transport'

/** Translation options supplied when rendering a kernel error. */
export interface ErrorTranslationOptions {
  [key: string]: unknown
  /** Returned when the catalogue has no entry for the code. */
  defaultValue: string
}

/** Minimal translation function used when rendering a kernel error. */
export type ErrorTranslator = (
  key: string,
  options: ErrorTranslationOptions,
) => string

/** A failure as the interface shows it: a summary plus the kernel's wording. */
export interface ErrorNotice {
  /** Translated summary, or the raw message when the code is unknown. */
  message: string
  /** Raw kernel wording, shown under "Details"; `null` when it is the message. */
  details: string | null
}

/** The message carried by any thrown value. */
export function rawMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

/**
 * Turns a rejected kernel call into a toast.
 *
 * Kernel errors carry a stable `ApiError.code`, which maps to `errors.<code>`
 * and moves the kernel's own wording — a path, a table name, a glyph handle —
 * under "Details". A code the catalogue does not know (a newer kernel, a
 * transport-level failure) falls back to that raw message, so nobody is left
 * looking at an untranslated identifier.
 */
export function errorNotice(error: unknown, t: ErrorTranslator): ErrorNotice {
  const message = rawMessage(error)
  if (!(error instanceof ApiCallError)) return { message, details: null }
  const translated = t(`errors.${error.code}`, { defaultValue: '' })
  return translated === ''
    ? { message, details: null }
    : { message: translated, details: message }
}
