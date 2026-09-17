import { describe, expect, it } from 'vitest'
import type { PositionedGlyph } from './textLayout'
import { identifyTextGaps, hitTextGap } from './textGaps'

function glyph(name: string, byte: number, x = byte * 600): PositionedGlyph {
  return {
    compiledGid: 91,
    documentGid: byte + 7,
    name,
    editable: true,
    x,
    y: 0,
    xAdvance: 600,
    yAdvance: 0,
    sourceByte: byte,
    sourceKind: 'literal',
  }
}

describe('editable shaped gaps', () => {
  it('uses exact glyph names and UTF-8 cluster boundaries for simple LTR pairs', () => {
    const gaps = identifyTextGaps(
      'ÁV',
      [glyph('Aacute', 0), glyph('V', 2, 520)],
      'ltr',
    )
    expect(gaps).toEqual([
      expect.objectContaining({
        index: 0,
        left: 'Aacute',
        right: 'V',
        x: 520,
        reason: null,
      }),
    ])
  })

  it('rejects ligatures, shared mark clusters, explicit names and unresolved glyphs', () => {
    expect(
      identifyTextGaps('fiV', [glyph('fi', 0), glyph('V', 2)], 'ltr')[0]
        ?.reason,
    ).toBe('cluster')
    const marked = [
      glyph('A', 0),
      { ...glyph('acutecomb', 0, 400), xAdvance: 0 },
      glyph('V', 3),
    ]
    expect(
      identifyTextGaps('A\u0301V', marked, 'ltr').every(
        (gap) => gap.reason === 'cluster',
      ),
    ).toBe(true)
    const named = [
      { ...glyph('A', 0), sourceKind: 'named' as const },
      { ...glyph('V', 3), sourceKind: 'named' as const },
    ]
    expect(identifyTextGaps('/A /V', named, 'ltr')[0]?.reason).toBe('named')
    expect(
      identifyTextGaps(
        'AV',
        [glyph('A', 0), { ...glyph('V', 1), documentGid: null }],
        'ltr',
      )[0]?.reason,
    ).toBe('unresolved')
  })

  it('rejects RTL, descending clusters and a trailing multi-character ligature', () => {
    const av = [glyph('A', 0), glyph('V', 1)]
    expect(identifyTextGaps('AV', av, 'rtl')[0]?.reason).toBe('rtl')
    expect(identifyTextGaps('AV', [...av].reverse(), 'ltr')[0]?.reason).toBe(
      'cluster',
    )
    expect(
      identifyTextGaps('Afi', [glyph('A', 0), glyph('fi', 1)], 'ltr')[0]
        ?.reason,
    ).toBe('cluster')
  })

  it('accepts a simple pair after an unrelated ligature and escaped slash', () => {
    const gaps = identifyTextGaps(
      'fi//AV',
      [glyph('fi', 0), glyph('slash', 2), glyph('A', 4), glyph('V', 5)],
      'ltr',
    )
    expect(gaps.map((gap) => gap.reason)).toEqual(['cluster', null, null])
  })

  it('hit tests a fixed screen-width strip without moving shaped positions', () => {
    const gaps = identifyTextGaps(
      'AV',
      [glyph('A', 0), glyph('V', 1, 520)],
      'ltr',
    )
    expect(hitTextGap(gaps, { x: 530, y: 300 }, 1000, 0.5)).toBe(0)
    expect(hitTextGap(gaps, { x: 540, y: 300 }, 1000, 0.5)).toBeNull()
    expect(hitTextGap(gaps, { x: 520, y: 900 }, 1000, 0.5)).toBeNull()
    expect(
      hitTextGap(
        [{ ...gaps[0]!, reason: 'cluster' }],
        { x: 520, y: 0 },
        1000,
        1,
      ),
    ).toBeNull()
    expect(gaps[0]?.x).toBe(520)
  })
})
