import { describe, expect, it } from 'vitest'

import { makeSummary } from '../test/mockTransport'

import { formatsFor, suggestPath, warningGlyphNames } from './exportFormats'

describe('export format helpers', () => {
  it('offers Designspace only for a multiple-master document', () => {
    expect(formatsFor(1)).toEqual(['Ttf', 'Woff', 'Woff2', 'Ufo', 'Fcf'])
    expect(formatsFor(3)).toContain('Designspace')
  })

  it('suggests the document’s own location and stem', () => {
    expect(
      suggestPath(makeSummary({ path: '/fonts/DejaVuSans.ttf' }), 'Woff2'),
    ).toBe('/fonts/DejaVuSans.woff2')
    expect(
      suggestPath(
        makeSummary({
          path: null,
          family_name: 'Workshop Sans',
          style_name: 'Bold',
        }),
        'Ttf',
      ),
    ).toBe('WorkshopSans-Bold.ttf')
  })

  it('reads the affected glyphs out of a hinting warning', () => {
    expect(
      warningGlyphNames({
        code: 'hinting-dropped',
        message:
          'TrueType instructions were dropped because these glyphs changed: A, B, a.sc',
      }),
    ).toEqual(['A', 'B', 'a.sc'])
  })

  it('claims no glyphs from a warning that does not list any', () => {
    expect(
      warningGlyphNames({
        code: 'hinting-dropped',
        message:
          'TrueType instructions were dropped because the original outline bytes are unavailable',
      }),
    ).toEqual([])
    expect(
      warningGlyphNames({
        code: 'value-clamped',
        message:
          'glyf: clamped 3 outline coordinate values to the OpenType range',
      }),
    ).toEqual([])
  })
})
