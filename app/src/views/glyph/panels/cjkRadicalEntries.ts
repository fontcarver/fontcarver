/**
 * Turning the kernel's radical library into the panel's pickable entries.
 *
 * Everything the radical library shows comes from data: the radicals, their
 * written forms and their stroke counts are derived from the pinned Unihan
 * `kRSUnicode`/`kTotalStrokes` values, and the component list is what this
 * document's layers actually reference. This module is the pure half — which
 * entries exist, how a query narrows them, and what each one contributes to the
 * IDS input — so the component is only markup and requests.
 */

import type { CjkRadicalLibrary } from '../../../api'

/** One pickable ideograph or component of the radical library. */
export interface RadicalEntry {
  /** Stable React key, unique across both tabs. */
  key: string
  /** Unicode scalar of the ideograph, or `null` for an unencoded component. */
  unicode: number | null
  /** Document glyph that carries this form, or `null` when the font lacks it. */
  glyph: string | null
  /** Kangxi radical number, or `null` for a component entry. */
  radical: number | null
  /** Recorded total stroke count, when the pinned source has one. */
  strokes: number | null
  /** Document glyphs using this radical, or references naming this component. */
  count: number
  /** Whether this is the Kangxi Radicals block character of its radical. */
  kangxi: boolean
}

/** The character an entry displays, or `null` for an unencoded component. */
export function entryChar(entry: RadicalEntry): string | null {
  return entry.unicode === null ? null : String.fromCodePoint(entry.unicode)
}

/** `U+XXXX` spelling of an entry's scalar, or an empty string. */
export function entryCodepoint(entry: RadicalEntry): string {
  if (entry.unicode === null) return ''
  return `U+${entry.unicode.toString(16).toUpperCase().padStart(4, '0')}`
}

/**
 * Every radical form, in radical order.
 *
 * Each radical contributes its Kangxi Radicals block character first — the
 * character a radical index is normally written with — and then the unified
 * ideographs that write it, so 人 and 亻 are separate picks because they are
 * separate glyphs.
 */
export function radicalEntries(
  library: CjkRadicalLibrary,
): readonly RadicalEntry[] {
  const entries: RadicalEntry[] = []
  for (const radical of library.radicals) {
    entries.push({
      key: `r${radical.number}:k`,
      unicode: radical.kangxi,
      glyph: radical.kangxi_glyph,
      radical: radical.number,
      strokes: radical.forms[0]?.strokes ?? null,
      count: radical.members,
      kangxi: true,
    })
    for (const form of radical.forms) {
      entries.push({
        key: `r${radical.number}:${form.unicode}`,
        unicode: form.unicode,
        glyph: form.glyph,
        radical: radical.number,
        strokes: form.strokes,
        count: radical.members,
        kangxi: false,
      })
    }
  }
  return entries
}

/** Every glyph this document already uses as a component, most used first. */
export function componentEntries(
  library: CjkRadicalLibrary,
): readonly RadicalEntry[] {
  return library.components.map((component) => ({
    key: `c:${component.name}`,
    unicode: component.unicode,
    glyph: component.name,
    radical: null,
    strokes: null,
    count: component.uses,
    kangxi: false,
  }))
}

/**
 * The entries a query and the "only forms in this font" switch leave.
 *
 * A number matches a radical number or a stroke count, a single character
 * matches that ideograph, and anything else is a case-insensitive substring of
 * the glyph name or of the `U+XXXX` spelling.
 */
export function filterEntries(
  entries: readonly RadicalEntry[],
  query: string,
  presentOnly: boolean,
): readonly RadicalEntry[] {
  const text = query.trim()
  const lower = text.toLowerCase()
  const numeric = /^[0-9]+$/.test(text) ? Number(text) : null
  const single = [...text].length === 1 ? text : null
  return entries.filter((entry) => {
    if (presentOnly && entry.glyph === null) return false
    if (!text) return true
    if (numeric !== null)
      return entry.radical === numeric || entry.strokes === numeric
    if (single !== null && entryChar(entry) === single) return true
    return (
      (entry.glyph !== null && entry.glyph.toLowerCase().includes(lower)) ||
      entryCodepoint(entry).toLowerCase().includes(lower)
    )
  })
}

/**
 * What picking an entry appends to the IDS input.
 *
 * A form the font already has is referenced by name, which is unambiguous; one
 * it does not have is referenced by its character, so the sequence still reads
 * correctly and the resolver reports the missing glyph instead of guessing.
 */
export function idsToken(entry: RadicalEntry): string {
  if (entry.glyph !== null) return `[${entry.glyph}]`
  return entryChar(entry) ?? ''
}

/** The distinct glyph names of one page, for a single outline request. */
export function pageGlyphNames(
  page: readonly RadicalEntry[],
): readonly string[] {
  return [
    ...new Set(
      page
        .map((entry) => entry.glyph)
        .filter((name): name is string => name !== null),
    ),
  ]
}
