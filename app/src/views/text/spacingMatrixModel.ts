/**
 * Pure helpers for the Spacing panel's metrics matrix, beam and cut-off band.
 *
 * The matrix is a spreadsheet over the glyphs of the shaped run: every cell
 * accepts a number *or* a metrics formula, which is what makes it an authoring
 * surface rather than a readout. The rules for deciding which of those a typed
 * cell is, and for the two display bands, live here so they can be tested
 * without a document.
 */

import type { SpacingRow } from '../../api'
import { SPACING_MATRIX_LIMIT } from './spacingLimits'
import type { PositionedGlyph } from './textLayout'

/** Which metric one matrix column edits. */
export type SpacingField = 'left' | 'right' | 'width'

/** How a typed cell should be applied. */
export type SpacingCellIntent =
  | { kind: 'unchanged' }
  | { kind: 'number'; value: number }
  | { kind: 'formula'; key: string }
  | { kind: 'clear' }

/** Strict finite decimal; anything else is treated as a formula. */
const NUMBER = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/u

/**
 * The distinct, resolvable glyph names of a run, in first-appearance order.
 *
 * A proof repeats letters, and a matrix row is per glyph, not per occurrence:
 * editing `n` twice in `nnn` would otherwise send the same command three times.
 * The list is bounded so one request can never grow with the text.
 */
export function runGlyphNames(
  glyphs: readonly PositionedGlyph[],
  limit = SPACING_MATRIX_LIMIT,
): string[] {
  const names: string[] = []
  const seen = new Set<string>()
  for (const glyph of glyphs) {
    if (glyph.documentGid === null || !glyph.name || seen.has(glyph.name))
      continue
    seen.add(glyph.name)
    names.push(glyph.name)
    if (names.length >= limit) break
  }
  return names
}

/** The formula shown in a cell, with `==` marking a layer-local one. */
export function displayedKey(
  row: Pick<SpacingRow, 'glyph_keys' | 'layer_keys'>,
  field: SpacingField,
): string | null {
  const local = row.layer_keys?.[field]
  if (local != null)
    return local.startsWith('==')
      ? local
      : local.startsWith('=')
        ? `=${local}`
        : `==${local}`
  return row.glyph_keys[field] ?? null
}

/** The stored number a cell shows when no formula applies. */
export function storedValue(
  row: SpacingRow,
  field: SpacingField,
): number | null {
  if (field === 'width') return row.width
  return field === 'left' ? row.lsb : row.rsb
}

/** What a cell shows: its formula if it has one, otherwise its stored value. */
export function cellText(row: SpacingRow, field: SpacingField): string {
  const key = displayedKey(row, field)
  if (key !== null) return key
  const value = storedValue(row, field)
  return value === null ? '' : String(Math.round(value * 10) / 10)
}

/**
 * Decides what a typed cell means.
 *
 * A plain number is a value; anything else is a formula, which is how the
 * requirement spells `n` and `=n` as the same thing. Clearing a cell that holds
 * a formula removes the formula and leaves the stored value alone; clearing a
 * cell that holds a bare number changes nothing, because a sidebearing has no
 * empty state.
 */
export function spacingCellIntent(
  row: SpacingRow,
  field: SpacingField,
  raw: string,
): SpacingCellIntent {
  const text = raw.trim()
  if (text === cellText(row, field)) return { kind: 'unchanged' }
  if (text === '')
    return displayedKey(row, field) === null
      ? { kind: 'unchanged' }
      : { kind: 'clear' }
  if (NUMBER.test(text)) {
    const value = Number(text)
    return Number.isFinite(value)
      ? { kind: 'number', value }
      : { kind: 'unchanged' }
  }
  return { kind: 'formula', key: text.startsWith('=') ? text : `=${text}` }
}

/** The beam margins of every measured row, keyed by glyph name. */
export function beamMargins(
  rows: readonly SpacingRow[],
): Map<string, { left: number; right: number }> {
  const margins = new Map<string, { left: number; right: number }>()
  for (const row of rows)
    if (row.beam_left !== null && row.beam_right !== null)
      margins.set(row.glyph, { left: row.beam_left, right: row.beam_right })
  return margins
}

/** Per-tab display state of the Spacing panel, the beam and the cut-off band. */
export interface SpacingViewState {
  /** Whether the matrix panel is expanded, which is also what it fetches for. */
  open: boolean
  beam: boolean
  beamHeight: number
  cutOff: boolean
  cutOffFrom: number
  cutOffTo: number
}

/**
 * Fills in whatever the tab has not chosen yet from the font's em size.
 *
 * A beam is most useful where the ink is, so it starts a quarter of an em above
 * the baseline, and the cut-off band starts as the lower half of the em: both
 * are immediately meaningful for a Latin lower case and are then dragged.
 */
export function resolveSpacingView(
  stored: Partial<SpacingViewState> | undefined,
  unitsPerEm: number,
): SpacingViewState {
  const em = Number.isFinite(unitsPerEm) && unitsPerEm > 0 ? unitsPerEm : 1000
  return {
    open: stored?.open ?? false,
    beam: stored?.beam ?? false,
    beamHeight: stored?.beamHeight ?? Math.round(em * 0.25),
    cutOff: stored?.cutOff ?? false,
    cutOffFrom: stored?.cutOffFrom ?? Math.round(em * 0.05),
    cutOffTo: stored?.cutOffTo ?? Math.round(em * 0.55),
  }
}
