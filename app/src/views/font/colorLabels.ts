/**
 * The twelve predefined glyph colour labels. The indexes are the ones stored in
 * the document (`GlyphCell.color_label`) and used by scripts and smart filters,
 * so they are fixed and identical in both themes.
 */

/** One colour label: its stored index, swatch and i18n key. */
export interface ColorLabel {
  readonly index: number
  readonly swatch: string
  readonly key: string
}

/** Colour labels in index order. */
export const COLOR_LABELS: readonly ColorLabel[] = [
  { index: 0, swatch: '#d0433a', key: 'red' },
  { index: 1, swatch: '#e08a2e', key: 'orange' },
  { index: 2, swatch: '#9a6a3f', key: 'brown' },
  { index: 3, swatch: '#d9bb2a', key: 'yellow' },
  { index: 4, swatch: '#86c14a', key: 'lightGreen' },
  { index: 5, swatch: '#3f8f45', key: 'darkGreen' },
  { index: 6, swatch: '#45b3c4', key: 'cyan' },
  { index: 7, swatch: '#3a6ea5', key: 'blue' },
  { index: 8, swatch: '#8a5cb0', key: 'purple' },
  { index: 9, swatch: '#d478a8', key: 'pink' },
  { index: 10, swatch: '#b8b8bd', key: 'lightGray' },
  { index: 11, swatch: '#6b6b74', key: 'darkGray' },
] as const

/** The swatch of one colour-label index, or `null` when it is unknown. */
export function colorLabelSwatch(index: number | null): string | null {
  if (index === null) return null
  return COLOR_LABELS[index]?.swatch ?? null
}

/** The i18n key of one colour-label index. */
export function colorLabelKey(index: number): string {
  return COLOR_LABELS[index]?.key ?? 'unknown'
}
