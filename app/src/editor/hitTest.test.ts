import { describe, expect, it } from 'vitest'

import { Viewport } from '../canvas/Viewport'
import { makeContour, makePoint } from '../test/fontFixtures'

import { HIT_RADII, hitTest, type HitTestInput } from './hitTest'
import { buildEditOutline } from './outlineGeometry'
import { EMPTY_SELECTION, selectOne } from './selection'

/** A viewport at 1 CSS pixel per font unit with the origin at (0, 300). */
function viewportAt(dpr = 1): Viewport {
  const viewport = new Viewport()
  viewport.setSize(400, 400, dpr)
  viewport.scale = 1
  viewport.offset = { x: 0, y: 300 }
  return viewport
}

// A square with one cubic segment on top: nodes at the corners, two controls
// bulging above the top edge, so nodes, handles and segments can all be aimed
// at independently.
const square = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(240, 120, 'OffCurve'),
  makePoint(-40, 120, 'OffCurve'),
  makePoint(0, 0, 'Curve'),
])

const anchors = [{ name: 'top', x: 100, y: 200 }]
const guides = [
  {
    guide: { x: 0, y: 150, angle: 0, name: 'mid', locked: false, color: null },
    scope: 'layer' as const,
    index: 0,
  },
]

function input(overrides: Partial<HitTestInput> = {}): HitTestInput {
  return {
    outline: buildEditOutline([square]),
    anchors,
    guides,
    components: [],
    viewport: viewportAt(),
    selection: EMPTY_SELECTION,
    ...overrides,
  }
}

describe('hitTest', () => {
  it('publishes the design-spec §4.2 radii', () => {
    expect(HIT_RADII).toEqual({
      node: 5,
      handle: 5,
      segment: 4,
      anchor: 6,
      guide: 4,
      component: 4,
    })
  })

  it('hits a node within 5 CSS pixels and misses beyond it', () => {
    // Font (200, 0) maps to screen (200, 300).
    expect(hitTest(input(), { x: 204, y: 300 })?.target).toEqual({
      kind: 'node',
      contour: 0,
      point: 1,
    })
    expect(hitTest(input(), { x: 206.5, y: 300 })?.target.kind).not.toBe('node')
  })

  it('measures the tolerance in CSS pixels whatever the device pixel ratio', () => {
    const at = { x: 204, y: 300 }
    const one = hitTest(input({ viewport: viewportAt(1) }), at)
    const two = hitTest(input({ viewport: viewportAt(2) }), at)
    expect(two?.target).toEqual(one?.target)
  })

  it('scales the tolerance with the zoom', () => {
    const zoomed = viewportAt()
    zoomed.scale = 4
    zoomed.offset = { x: 0, y: 300 }
    // Font (200, 0) is now at screen (800, 300); 4 px away is still a hit.
    expect(
      hitTest(input({ viewport: zoomed }), { x: 804, y: 300 })?.target,
    ).toEqual({ kind: 'node', contour: 0, point: 1 })
  })

  it('prefers a node to a handle when both are in range', () => {
    // A contour whose handle sits two units from a node.
    const overlapping = makeContour([
      makePoint(0, 0),
      makePoint(100, 0, 'Line'),
      makePoint(102, 0, 'OffCurve'),
      makePoint(0, 60, 'Curve'),
    ])
    const hit = hitTest(input({ outline: buildEditOutline([overlapping]) }), {
      x: 101,
      y: 300,
    })
    expect(hit?.target).toEqual({ kind: 'node', contour: 0, point: 1 })
  })

  it('prefers an already selected node to an unselected one', () => {
    const twin = makeContour([
      makePoint(0, 0),
      makePoint(100, 0, 'Line'),
      makePoint(103, 0, 'Line'),
    ])
    const selection = selectOne({ kind: 'node', contour: 0, point: 2 })
    const hit = hitTest(
      input({ outline: buildEditOutline([twin]), selection }),
      { x: 101, y: 300 },
    )
    expect(hit?.target).toEqual({ kind: 'node', contour: 0, point: 2 })
  })

  it('hits an anchor before a guide and a guide before a segment', () => {
    // The anchor sits at font (100, 200) → screen (100, 100).
    expect(hitTest(input(), { x: 100, y: 100 })?.target).toEqual({
      kind: 'anchor',
      index: 0,
    })
    // The guide runs along y = 150 → screen y = 150, away from any node.
    expect(hitTest(input(), { x: 60, y: 150 })?.target).toEqual({
      kind: 'guide',
      index: 0,
      scope: 'layer',
    })
  })

  it('returns the guide index within its scope from a mixed list', () => {
    const mixed = [
      {
        guide: { ...guides[0]!.guide, y: 50 },
        scope: 'master' as const,
        index: 0,
      },
      { ...guides[0]!, index: 0 },
    ]
    expect(
      hitTest(input({ guides: mixed }), { x: 60, y: 150 })?.target,
    ).toEqual({ kind: 'guide', index: 0, scope: 'layer' })
  })

  it('hits a segment and reports where along it the pointer landed', () => {
    // The bottom edge runs from font (0, 0) to (200, 0) → screen y 300.
    const hit = hitTest(input(), { x: 100, y: 302 })
    expect(hit?.target).toEqual({ kind: 'segment', contour: 0, segment: 0 })
    expect(hit?.t).toBeCloseTo(0.5, 2)
    expect(hit?.point.x).toBeCloseTo(100, 1)
  })

  it('ignores handles when asked, for the Alt marquee and Alt hover', () => {
    // Font (240, 120) → screen (240, 180) is the first control point.
    expect(hitTest(input(), { x: 240, y: 180 })?.target).toEqual({
      kind: 'handle',
      contour: 0,
      point: 2,
    })
    expect(
      hitTest(input(), { x: 240, y: 180 }, { ignoreHandles: true })?.target
        .kind,
    ).not.toBe('handle')
  })

  it('finds nothing on empty canvas', () => {
    expect(hitTest(input(), { x: 380, y: 20 })).toBeNull()
  })
})
