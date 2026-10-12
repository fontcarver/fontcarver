import { describe, expect, it } from 'vitest'

import type { Anchor, GlyphCell, GlyphDetail } from '../../../api'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
} from '../../../test/fontFixtures'
import {
  markCloudViewBox,
  matchMarkAttachment,
  type MarkAttachment,
} from './markCloudGeometry'

const baseAnchor: Anchor = { name: 'top', x: 300, y: 700 }
const markAnchor: Anchor = { name: '_top', x: 50, y: 20 }
const markPath = 'M0 20L100 20L100 100Z'

function candidate(): { detail: GlyphDetail; cell: GlyphCell } {
  return {
    detail: makeGlyphDetail(
      makeGlyph({
        name: 'acutecomb',
        layers: [makeLayer({ anchors: [markAnchor] })],
      }),
      markPath,
    ),
    cell: makeCell({
      name: 'acutecomb',
      category: 'Mark',
      bbox: [0, 20, 100, 100],
      path: 'M0 0L20 20Z',
    }),
  }
}

describe('mark attachment matching', () => {
  it('aligns the underscore mark anchor with the base anchor in font space', () => {
    const { detail, cell } = candidate()
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toEqual({
      name: 'acutecomb',
      path: markPath,
      bbox: [0, 20, 100, 100],
      anchor: markAnchor,
      dx: 250,
      dy: 680,
    })
  })

  it('selects the exact master and its flattened path by layer ID', () => {
    const { detail, cell } = candidate()
    detail.glyph.layers.unshift(
      makeLayer({
        id: 'other',
        kind: { Master: 'm2' },
        anchors: [{ ...markAnchor, x: 999 }],
      }),
    )
    detail.paths.unshift({ layer: 'other', flattened: 'M999 0L999 999Z' })
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toMatchObject({
      path: markPath,
      dx: 250,
    })
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'missing')).toBeNull()
  })

  it('does not treat supplemental layers as the requested master layer', () => {
    const { detail, cell } = candidate()
    detail.glyph.layers[0]!.kind = { Background: 'm1' }
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toBeNull()
    detail.glyph.layers[0]!.kind = { Brace: { master: 'm1', location: {} } }
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toBeNull()
  })

  it.each(['', ' ', '_top', 'bottom', 'Top'])(
    'rejects unmatched base name %j',
    (name) => {
      const { detail, cell } = candidate()
      expect(
        matchMarkAttachment({ ...baseAnchor, name }, detail, cell, 'm1'),
      ).toBeNull()
    },
  )

  it('uses the requested master grid path only when the detail path is missing', () => {
    const { detail, cell } = candidate()
    detail.paths = [{ layer: 'unrelated', flattened: markPath }]
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')?.path).toBe(
      cell.path,
    )
    cell.path = null
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toBeNull()
  })

  it('does not substitute a stale grid path for an explicitly empty detail path', () => {
    const { detail, cell } = candidate()
    detail.paths[0]!.flattened = '  '
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toBeNull()
  })

  it('rejects nonfinite anchor coordinates and overflowing translations', () => {
    const { detail, cell } = candidate()
    expect(
      matchMarkAttachment({ ...baseAnchor, x: NaN }, detail, cell, 'm1'),
    ).toBeNull()
    detail.glyph.layers[0]!.anchors = [{ ...markAnchor, y: Infinity }]
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')).toBeNull()
    detail.glyph.layers[0]!.anchors = [{ ...markAnchor, x: -Number.MAX_VALUE }]
    expect(
      matchMarkAttachment(
        { ...baseAnchor, x: Number.MAX_VALUE },
        detail,
        cell,
        'm1',
      ),
    ).toBeNull()
  })

  it('keeps renderable marks with missing or invalid bounds', () => {
    const { detail, cell } = candidate()
    cell.bbox = null
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')?.bbox).toBeNull()
    cell.bbox = [0, 0, Infinity, 100]
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')?.bbox).toBeNull()
    cell.bbox = [100, 0, 0, 100]
    expect(matchMarkAttachment(baseAnchor, detail, cell, 'm1')?.bbox).toBeNull()
  })
})

function attachment(overrides: Partial<MarkAttachment> = {}): MarkAttachment {
  return {
    name: 'acutecomb',
    path: markPath,
    bbox: [0, 20, 100, 100],
    anchor: markAnchor,
    dx: 250,
    dy: 680,
    ...overrides,
  }
}

describe('mark cloud framing', () => {
  it('frames the base and translated mark with padding in SVG coordinates', () => {
    expect(markCloudViewBox([0, 0, 600, 700], 600, 1000, attachment())).toBe(
      '-60 -840 720 900',
    )
  })

  it('includes marks below the baseline and outside the base advance', () => {
    expect(
      markCloudViewBox(
        [0, 0, 600, 700],
        600,
        1000,
        attachment({ dx: -150, dy: -200 }),
      ),
    ).toBe('-210 -760 870 1000')
  })

  it('uses advance and UPEM when the base has no outline bounds', () => {
    expect(markCloudViewBox(null, 600, 1000, attachment())).toBe(
      '-60 -860 720 1120',
    )
  })

  it('reserves space around the translated anchor when mark bounds are missing', () => {
    expect(
      markCloudViewBox([0, 0, 600, 700], 600, 1000, attachment({ bbox: null })),
    ).toBe('-60 -1010 720 1070')
  })

  it('returns a positive finite viewport for invalid metrics and bounds', () => {
    const viewBox = markCloudViewBox(
      [NaN, 0, 100, 100],
      Infinity,
      0,
      attachment({ bbox: [100, 0, 0, 100], dx: NaN }),
    )
      .split(' ')
      .map(Number)
    expect(viewBox).toHaveLength(4)
    expect(viewBox.every(Number.isFinite)).toBe(true)
    expect(viewBox[2]).toBeGreaterThan(0)
    expect(viewBox[3]).toBeGreaterThan(0)
  })

  it('gives zero-area bounds a nonzero viewport', () => {
    const viewBox = markCloudViewBox(
      [0, 0, 0, 0],
      0,
      1000,
      attachment({ bbox: [0, 0, 0, 0], dx: 0, dy: 0 }),
    )
      .split(' ')
      .map(Number)
    expect(viewBox).toEqual([-60, -60, 120, 120])
  })
})
