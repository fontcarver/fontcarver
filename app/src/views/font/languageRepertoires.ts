/** A bounded letter repertoire, not a claim of complete language support. */
export interface LanguageRepertoire {
  readonly id: string
  readonly scalars: readonly number[]
}

// CLDR 48 base exemplar characters; Unicode License v3,
// Copyright © 2004–2025 Unicode, Inc. See THIRD_PARTY.md and
// crates/fc-glyphdata/data/UNICODE-LICENSE.txt.
// Source: https://github.com/unicode-org/cldr/tree/release-48/common/main
// Uppercase counterparts are added using the locale's casing rules. Exemplar
// letters alone do not test combining marks, shaping, punctuation or outlines.
const BASE_EXEMPLARS: Readonly<Record<string, string>> = {
  en: 'abcdefghijklmnopqrstuvwxyz',
  fr: 'aàâæbcçdeéèêëfghiîïjklmnoôœpqrstuùûüvwxyÿz',
  de: 'aäbcdefghijklmnoöpqrsßtuüvwxyz',
  es: 'aábcdeéfghiíjklmnñoópqrstuúüvwxyz',
  it: 'aàbcdeéèfghiìjklmnoòpqrstuùvwxyz',
  pt: 'aáàâãbcçdeéêfghiíjklmnoóòôõpqrstuúvwxyz',
  pl: 'aąbcćdeęfghijklłmnńoóprsśtuwyzźż',
  tr: 'abcçdefgğhıiİjklmnoöprsştuüvyz',
  sv: 'aàbcdeéfghijklmnopqrstuvwxyzåäö',
}

/** Pinned CLDR exemplar letters plus their locale-sensitive uppercase forms. */
export const languageRepertoires: readonly LanguageRepertoire[] =
  Object.entries(BASE_EXEMPLARS).map(([id, letters]) => ({
    id,
    scalars: [
      ...new Set(
        Array.from(letters + letters.toLocaleUpperCase(id), (character) =>
          character.codePointAt(0)!,
        ),
      ),
    ].sort((a, b) => a - b),
  }))

/** Counts Unicode assignments, including secondary assignments, exactly once. */
export function languageCoverage(
  scalars: readonly number[],
  encoded: readonly number[],
): { present: number; total: number; missing: number[] } {
  const present = new Set(encoded)
  const repertoire = [...new Set(scalars)]
  const missing = repertoire.filter((scalar) => !present.has(scalar))
  return {
    present: repertoire.length - missing.length,
    total: repertoire.length,
    missing,
  }
}

/** Minimal inclusive ranges with no unintended characters between letters. */
export function scalarRanges(scalars: readonly number[]): [number, number][] {
  const ranges: [number, number][] = []
  for (const scalar of [...new Set(scalars)].sort((a, b) => a - b)) {
    const previous = ranges.at(-1)
    if (previous && previous[1] + 1 === scalar) previous[1] = scalar
    else ranges.push([scalar, scalar])
  }
  return ranges
}

/** An explicit scalar recipe avoids accidental glyph-name or range expansion. */
export function scalarRecipe(scalars: readonly number[]): string {
  return scalars
    .map((scalar) => `U+${scalar.toString(16).toUpperCase().padStart(4, '0')}`)
    .join(' ')
}
