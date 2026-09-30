import { describe, expect, it } from 'vitest'

import {
  makeCell,
  makeContour,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeMaster,
  makeMasterMetrics,
  makePoint,
} from '../../test/fontFixtures'

import {
  activeLayer,
  buildScene,
  formatUnicodes,
  layerMasterId,
  pointCount,
  sceneBox,
} from './glyphScene'

const master = makeMaster()

function sceneOf(overrides: Parameters<typeof buildScene>[0] | null = null) {
  return buildScene(
    overrides ?? {
      detail: makeGlyphDetail(),
      cell: makeCell(),
      master,
      masterId: 'm1',
      unitsPerEm: 1000,
    },
  )
}

describe('layerMasterId', () => {
  it('reads the master out of every layer kind', () => {
    expect(layerMasterId({ Master: 'm1' })).toBe('m1')
    expect(layerMasterId({ Brace: { master: 'm2', location: {} } })).toBe('m2')
    expect(
      layerMasterId({
        Bracket: {
          master: 'm3',
          axis: [119, 103, 104, 116],
          min: null,
          max: null,
        },
      }),
    ).toBe('m3')
    expect(layerMasterId({ Background: 'm4' })).toBe('m4')
    expect(layerMasterId({ Backup: { of: 'm5', time: 0 } })).toBe('m5')
    expect(layerMasterId({ Custom: 'sketch' })).toBeNull()
  })
})

describe('activeLayer', () => {
  it('prefers the master layer of the selected master', () => {
    const glyph = makeGlyph({
      layers: [
        makeLayer({ id: 'bg', kind: { Background: 'm1' }, name: 'Background' }),
        makeLayer({ id: 'l2', kind: { Master: 'm2' }, name: 'Bold' }),
        makeLayer({ id: 'l1', kind: { Master: 'm1' }, name: 'Regular' }),
      ],
    })
    expect(activeLayer(glyph, 'm2')?.id).toBe('l2')
    expect(activeLayer(glyph, 'm1')?.id).toBe('l1')
  })

  it('falls back to the first master layer, then to any layer', () => {
    const glyph = makeGlyph({
      layers: [makeLayer({ id: 'l9', kind: { Master: 'm9' } })],
    })
    expect(activeLayer(glyph, 'missing')?.id).toBe('l9')

    const custom = makeGlyph({
      layers: [makeLayer({ id: 'c1', kind: { Custom: 'sketch' } })],
    })
    expect(activeLayer(custom, 'm1')?.id).toBe('c1')

    expect(activeLayer(makeGlyph({ layers: [] }), 'm1')).toBeNull()
  })
})

describe('buildScene', () => {
  it('computes the sidebearings from the cell bounds and the layer width', () => {
    const scene = sceneOf()
    expect(scene.metrics.width).toBe(600)
    expect(scene.metrics.lsb).toBe(40)
    expect(scene.metrics.rsb).toBe(40)
    expect(scene.bbox).toEqual({ x0: 40, y0: 0, x1: 560, y1: 700 })
  })

  it('reports no sidebearings for a glyph with no outline', () => {
    const scene = sceneOf({
      detail: makeGlyphDetail(),
      cell: makeCell({ bbox: null }),
      master,
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(scene.metrics.lsb).toBeNull()
    expect(scene.metrics.rsb).toBeNull()
  })

  it('takes the vertical metrics from the master and falls back sensibly', () => {
    const scene = sceneOf()
    expect(scene.metrics.ascender).toBe(800)
    expect(scene.metrics.capHeight).toBe(700)

    const bare = sceneOf({
      detail: makeGlyphDetail(),
      cell: null,
      master: null,
      masterId: null,
      unitsPerEm: 1000,
    })
    expect(bare.metrics.ascender).toBe(800)
    expect(bare.metrics.descender).toBe(-200)
    expect(bare.metrics.xHeight).toBe(500)
  })

  it('carries the vertical origin and advance a CJK layer defines', () => {
    const glyph = makeGlyph({
      layers: [makeLayer({ vertical_origin: 880, vertical_width: 1000 })],
    })
    const scene = sceneOf({
      detail: makeGlyphDetail(glyph),
      cell: makeCell(),
      master,
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(scene.metrics.verticalOrigin).toBe(880)
    expect(scene.metrics.verticalWidth).toBe(1000)
  })

  it('picks the flattened path that belongs to the shown layer', () => {
    const glyph = makeGlyph({
      layers: [
        makeLayer({ id: 'l1', kind: { Master: 'm1' } }),
        makeLayer({ id: 'l2', kind: { Master: 'm2' } }),
      ],
    })
    const detail = {
      glyph,
      paths: [
        { layer: 'l1', flattened: 'M0 0Z' },
        { layer: 'l2', flattened: 'M1 1Z' },
      ],
    }
    expect(
      buildScene({
        detail,
        cell: null,
        master,
        masterId: 'm2',
        unitsPerEm: 1000,
      }).flattened,
    ).toBe('M1 1Z')
  })

  it('merges master guides ahead of layer guides', () => {
    const guide = {
      x: 0,
      y: 100,
      angle: 0,
      name: 'g',
      locked: false,
      color: null,
      anchor: 'Origin' as const,
    }
    const scene = sceneOf({
      detail: makeGlyphDetail(
        makeGlyph({ layers: [makeLayer({ guides: [guide] })] }),
      ),
      cell: null,
      master: makeMaster({ guides: [guide] }),
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(scene.guides.map((entry) => entry.scope)).toEqual([
      'master',
      'layer',
    ])
  })

  it('resolves an anchored guide against the ink edges it follows', () => {
    // The fixture cell's bounds are [40, 0, 560, 700] at advance 600.
    const anchored = (anchor: 'Origin' | 'Lsb' | 'Rsb', x: number) => ({
      x,
      y: 0,
      angle: 90,
      name: null,
      locked: false,
      color: null,
      anchor,
    })
    const scene = sceneOf({
      detail: makeGlyphDetail(
        makeGlyph({
          layers: [
            makeLayer({
              guides: [
                anchored('Origin', 120),
                anchored('Lsb', 30),
                anchored('Rsb', -30),
              ],
            }),
          ],
        }),
      ),
      cell: makeCell(),
      master,
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(scene.guides.map((entry) => entry.guide.x)).toEqual([120, 70, 530])
    // The stored offsets are untouched: the scene resolves, it does not write.
    expect(scene.layer?.guides.map((guide) => guide.x)).toEqual([120, 30, -30])
  })

  it('keeps an anchored guide absolute when the layer has no ink', () => {
    const scene = sceneOf({
      detail: makeGlyphDetail(
        makeGlyph({
          layers: [
            makeLayer({
              contours: [],
              guides: [
                {
                  x: 42,
                  y: 0,
                  angle: 90,
                  name: null,
                  locked: false,
                  color: null,
                  anchor: 'Lsb' as const,
                },
              ],
            }),
          ],
        }),
      ),
      cell: null,
      master,
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(scene.guides[0]?.guide.x).toBe(42)
  })
})

describe('sceneBox', () => {
  it('unions the advance box with the outline bounds', () => {
    const box = sceneBox(sceneOf())
    expect(box).toEqual({ x0: 0, y0: -200, x1: 600, y1: 800 })
  })

  it('grows for an outline that overshoots the advance', () => {
    const box = sceneBox(
      sceneOf({
        detail: makeGlyphDetail(),
        cell: makeCell({ bbox: [-80, -260, 900, 980] }),
        master,
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
    )
    expect(box).toEqual({ x0: -80, y0: -260, x1: 900, y1: 980 })
  })

  it('uses the em width for a zero-width glyph', () => {
    const box = sceneBox(
      sceneOf({
        detail: makeGlyphDetail(
          makeGlyph({ layers: [makeLayer({ width: 0 })] }),
        ),
        cell: null,
        master,
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
    )
    expect(box.x1).toBe(1000)
  })
})

describe('formatting helpers', () => {
  it('renders Unicode scalars in U+ notation', () => {
    expect(formatUnicodes([0x61])).toBe('U+0061')
    expect(formatUnicodes([0x4e00, 0x1f600])).toBe('U+4E00 U+1F600')
    expect(formatUnicodes([])).toBe('—')
  })

  it('counts every on-curve and off-curve point', () => {
    const scene = buildScene({
      detail: makeGlyphDetail(
        makeGlyph({
          layers: [
            makeLayer({
              contours: [
                makeContour([
                  makePoint(0, 0),
                  makePoint(10, 0, 'OffCurve'),
                  makePoint(10, 10, 'Curve'),
                ]),
                makeContour([makePoint(0, 0), makePoint(5, 5)]),
              ],
            }),
          ],
        }),
      ),
      cell: null,
      master,
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(pointCount(scene)).toBe(5)
  })
})

describe('master metrics overrides', () => {
  it('uses the master defaults for the vertical origin when the layer has none', () => {
    const scene = buildScene({
      detail: makeGlyphDetail(),
      cell: null,
      master: makeMaster({
        metrics: makeMasterMetrics({
          default_vertical_origin: 880,
          default_vertical_advance: 1000,
        }),
      }),
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    expect(scene.metrics.verticalOrigin).toBe(880)
    expect(scene.metrics.verticalWidth).toBe(1000)
  })
})
