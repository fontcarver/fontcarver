import { describe, expect, it } from 'vitest'

import type { GlyphId, MetricsKeys, SpacingRow } from '../../api'
import {
  beamMargins,
  cellText,
  displayedKey,
  resolveSpacingView,
  runGlyphNames,
  spacingCellIntent,
} from './spacingMatrix'
import type { PositionedGlyph } from './textLayout'

const NO_KEYS: MetricsKeys = {
  left: null,
  right: null,
  width: null,
  vertical_origin: null,
  vertical_width: null,
}

function row(overrides: Partial<SpacingRow> = {}): SpacingRow {
  return {
    glyph: 'n',
    layer: 'layer-1',
    width: 600,
    lsb: 40,
    rsb: 45,
    glyph_keys: NO_KEYS,
    layer_keys: null,
    resolved: null,
    key_error: null,
    out_of_sync: false,
    beam_left: null,
    beam_right: null,
    missing: false,
    locked: false,
    ...overrides,
  }
}

function glyph(name: string | null, gid: number | null): PositionedGlyph {
  return {
    compiledGid: gid,
    documentGid: gid === null ? null : (gid as GlyphId),
    name,
    editable: gid !== null,
    x: 0,
    y: 0,
    xAdvance: 500,
    yAdvance: 0,
    sourceByte: 0,
    sourceKind: 'literal',
  }
}

describe('runGlyphNames', () => {
  it('keeps the first appearance of each resolvable glyph', () => {
    expect(
      runGlyphNames([
        glyph('n', 1),
        glyph('o', 2),
        glyph('n', 1),
        glyph(null, 3),
        glyph('x', null),
      ]),
    ).toEqual(['n', 'o'])
  })

  it('never grows past its bound however long the text is', () => {
    const many = Array.from({ length: 50 }, (_, index) =>
      glyph(`g${index}`, index + 1),
    )
    expect(runGlyphNames(many, 8)).toHaveLength(8)
  })
})

describe('matrix cells', () => {
  it('shows the formula when there is one and the number otherwise', () => {
    expect(cellText(row(), 'left')).toBe('40')
    expect(cellText(row({ lsb: null }), 'left')).toBe('')
    expect(cellText(row(), 'width')).toBe('600')
    expect(
      cellText(row({ glyph_keys: { ...NO_KEYS, left: '=H' } }), 'left'),
    ).toBe('=H')
  })

  it('marks a layer-local formula with its scope prefix', () => {
    const local = row({
      glyph_keys: { ...NO_KEYS, left: '=H' },
      layer_keys: { ...NO_KEYS, left: '==O' },
    })
    expect(displayedKey(local, 'left')).toBe('==O')
    // An imported local formula stored without the prefix still reads as local.
    expect(
      displayedKey(row({ layer_keys: { ...NO_KEYS, left: '=O' } }), 'left'),
    ).toBe('==O')
    expect(displayedKey(row(), 'left')).toBeNull()
  })

  it('reads a plain number as a value and anything else as a formula', () => {
    expect(spacingCellIntent(row(), 'left', '55')).toEqual({
      kind: 'number',
      value: 55,
    })
    expect(spacingCellIntent(row(), 'left', '-12.5')).toEqual({
      kind: 'number',
      value: -12.5,
    })
    // The bare spelling of =n is accepted, as the requirement lists it.
    expect(spacingCellIntent(row(), 'left', 'H')).toEqual({
      kind: 'formula',
      key: '=H',
    })
    expect(spacingCellIntent(row(), 'left', '=H+10')).toEqual({
      kind: 'formula',
      key: '=H+10',
    })
  })

  it('does nothing when a cell is left as it was found', () => {
    expect(spacingCellIntent(row(), 'left', '40')).toEqual({
      kind: 'unchanged',
    })
    expect(spacingCellIntent(row(), 'left', ' 40 ')).toEqual({
      kind: 'unchanged',
    })
    // Emptying a plain number is not an edit: a sidebearing has no empty state.
    expect(spacingCellIntent(row(), 'left', '')).toEqual({ kind: 'unchanged' })
  })

  it('treats emptying a formula cell as clearing the formula', () => {
    const keyed = row({ glyph_keys: { ...NO_KEYS, left: '=H' } })
    expect(spacingCellIntent(keyed, 'left', '')).toEqual({ kind: 'clear' })
  })
})

describe('beam and cut-off state', () => {
  it('collects only the rows the beam actually crossed', () => {
    const margins = beamMargins([
      row({ glyph: 'n', beam_left: 40, beam_right: 42 }),
      row({ glyph: 'o', beam_left: null, beam_right: null }),
    ])
    expect([...margins.keys()]).toEqual(['n'])
    expect(margins.get('n')).toEqual({ left: 40, right: 42 })
  })

  it('derives usable defaults from the em size and keeps explicit choices', () => {
    expect(resolveSpacingView(undefined, 1000)).toEqual({
      open: false,
      beam: false,
      beamHeight: 250,
      cutOff: false,
      cutOffFrom: 50,
      cutOffTo: 550,
    })
    expect(resolveSpacingView(undefined, 2048).beamHeight).toBe(512)
    // A nonsensical em size must not produce NaN controls.
    expect(resolveSpacingView(undefined, 0).beamHeight).toBe(250)
    expect(
      resolveSpacingView({ beam: true, beamHeight: 0 }, 1000),
    ).toMatchObject({ beam: true, beamHeight: 0 })
  })
})
