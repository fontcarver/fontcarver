import { describe, expect, it } from 'vitest'

import { Viewport } from '../../canvas/Viewport'
import { makeContour, makePoint } from '../../test/fontFixtures'
import type { HitTestInput } from '../hitTest'
import { buildEditOutline } from '../outlineGeometry'
import { EMPTY_SELECTION, selectOne, unionSelection } from '../selection'

import { marqueeRect, marqueeSelection } from './marquee'

function viewport(): Viewport {
  const value = new Viewport()
  value.setSize(400, 400, 1)
  value.scale = 1
  value.offset = { x: 0, y: 300 }
  return value
}

// Two corners with a control point between them, plus one anchor.
const contour = makeContour([
  makePoint(0, 0),
  makePoint(100, 0, 'Line'),
  makePoint(150, 40, 'OffCurve'),
  makePoint(200, 100, 'Curve'),
])

function input(): HitTestInput {
  return {
    outline: buildEditOutline([contour]),
    anchors: [{ name: 'top', x: 50, y: 20 }],
    guides: [],
    components: [{ bbox: [300, 0, 360, 60] }],
    viewport: viewport(),
    selection: EMPTY_SELECTION,
  }
}

describe('marqueeRect', () => {
  it('normalises a drag in any direction', () => {
    expect(marqueeRect({ x: 120, y: 40 }, { x: 20, y: 90 })).toEqual({
      x0: 20,
      y0: 40,
      x1: 120,
      y1: 90,
    })
  })
})

describe('marqueeSelection', () => {
  it('selects every node, handle and anchor inside the rectangle', () => {
    // Font x 0…160, y 0…60 → screen x 0…160, y 240…300.
    const selection = marqueeSelection(
      input(),
      { x0: -10, y0: 235, x1: 160, y1: 305 },
      {},
    )
    expect([...selection.keys()].sort()).toEqual([
      'anchor:0',
      'handle:0:2',
      'node:0:0',
      'node:0:1',
    ])
  })

  it('ignores handles while Alt is held', () => {
    const selection = marqueeSelection(
      input(),
      { x0: -10, y0: 235, x1: 160, y1: 305 },
      { ignoreHandles: true },
    )
    expect([...selection.keys()].sort()).toEqual([
      'anchor:0',
      'node:0:0',
      'node:0:1',
    ])
  })

  it('selects a component only when its bounds are fully enclosed', () => {
    const partial = marqueeSelection(
      input(),
      { x0: 290, y0: 260, x1: 400, y1: 320 },
      {},
    )
    expect(partial.size).toBe(0)
    const whole = marqueeSelection(
      input(),
      { x0: 290, y0: 230, x1: 400, y1: 320 },
      {},
    )
    expect([...whole.keys()]).toEqual(['component:0'])
  })

  it('adds to the existing selection when Shift is held', () => {
    const existing = selectOne({ kind: 'node', contour: 0, point: 3 })
    const marquee = marqueeSelection(
      input(),
      { x0: -10, y0: 290, x1: 20, y1: 310 },
      {},
    )
    expect([...unionSelection(existing, marquee).keys()].sort()).toEqual([
      'node:0:0',
      'node:0:3',
    ])
  })

  it('selects nothing when the rectangle is empty', () => {
    expect(
      marqueeSelection(input(), { x0: 250, y0: 10, x1: 280, y1: 40 }, {}).size,
    ).toBe(0)
  })
})
