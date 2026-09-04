import { describe, expect, it } from 'vitest'

import { Viewport } from '../canvas/Viewport'
import { makeContour, makePoint } from '../test/fontFixtures'

import { buildEditOutline } from './outlineGeometry'
import { snapDelta, SNAP_DISTANCE, type SnapInput } from './snapping'

const contour = makeContour([
  makePoint(0, 0),
  makePoint(100, 0, 'Line'),
  makePoint(100, 500, 'Line'),
])

function input(overrides: Partial<SnapInput> = {}): SnapInput {
  const viewport = new Viewport()
  viewport.setSize(400, 400, 1)
  viewport.scale = 1
  viewport.offset = { x: 0, y: 300 }
  return {
    outline: buildEditOutline([contour]),
    anchors: [{ name: 'top', x: 300, y: 300 }],
    guides: [
      {
        guide: {
          x: 0,
          y: 250,
          angle: 0,
          name: 'mid',
          locked: false,
          color: null,
        },
        scope: 'layer',
      },
    ],
    metrics: {
      upem: 1000,
      ascender: 800,
      descender: -200,
      xHeight: 500,
      capHeight: 700,
      italicAngle: 0,
      width: 600,
      lsb: 0,
      rsb: 0,
      verticalOrigin: null,
      verticalWidth: null,
    },
    names: {
      node: 'NODE',
      handle: 'HANDLE',
      guide: 'GUIDE',
      baseline: 'baseline',
      xHeight: 'x-height',
      capHeight: 'cap height',
      ascender: 'ascender',
      descender: 'descender',
    },
    viewport,
    exclude: new Set(['0:2']),
    ...overrides,
  }
}

describe('snapDelta', () => {
  it("names its targets from the caller's bundle, never a literal", () => {
    // Every user-visible string in the editor arrives translated; the module
    // must not invent one of its own.
    const unnamed = input({
      guides: [
        {
          guide: {
            x: 0,
            y: 250,
            angle: 0,
            name: null,
            locked: false,
            color: null,
          },
          scope: 'layer',
        },
      ],
    })
    expect(snapDelta(unnamed, { x: 0, y: 0 }, [{ x: 300, y: 253 }]).label).toBe(
      'GUIDE',
    )
  })

  it('is 6 CSS pixels', () => {
    expect(SNAP_DISTANCE).toBe(6)
  })

  it('snaps onto another node, both axes at once', () => {
    // Dragging point 2 from (100, 500) by (−3, −497) lands near (97, 3).
    const match = snapDelta(input(), { x: -3, y: -497 }, [{ x: 97, y: 3 }])
    expect(match.delta).toEqual({ x: 0, y: -500 })
    expect(match.label).toBe('NODE')
    expect(match.point).toEqual({ x: 100, y: 0 })
  })

  it('prefers a node to an anchor at the same distance', () => {
    const match = snapDelta(input(), { x: 0, y: 0 }, [{ x: 102, y: 2 }])
    expect(match.point).toEqual({ x: 100, y: 0 })
  })

  it('snaps the vertical axis onto a metric line', () => {
    const match = snapDelta(input(), { x: 0, y: 0 }, [{ x: 300, y: 497 }])
    expect(match.delta.y).toBeCloseTo(3, 6)
    expect(match.label).toBe('x-height')
  })

  it('snaps onto a horizontal guide before a metric line', () => {
    const match = snapDelta(input(), { x: 0, y: 0 }, [{ x: 300, y: 253 }])
    expect(match.delta.y).toBeCloseTo(-3, 6)
    expect(match.label).toBe('mid')
  })

  it('reports an alignment line when a point shares an axis', () => {
    const match = snapDelta(input(), { x: 0, y: 0 }, [{ x: 98, y: 320 }])
    expect(match.alignedX).toBe(100)
    expect(match.delta.x).toBeCloseTo(2, 6)
  })

  it('leaves the delta alone when nothing is near', () => {
    const match = snapDelta(input(), { x: 7, y: 9 }, [{ x: 320, y: 333 }])
    expect(match.delta).toEqual({ x: 7, y: 9 })
    expect(match.label).toBeNull()
  })
})
