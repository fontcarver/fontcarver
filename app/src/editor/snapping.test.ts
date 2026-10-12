import { describe, expect, it } from 'vitest'

import { Viewport } from '../canvas/Viewport'
import { makeContour, makePoint } from '../test/fontFixtures'

import { buildEditOutline } from './outlineGeometry'
import {
  DEFAULT_SNAP_SETTINGS,
  NO_SNAP_DISTANCES,
  snapDelta,
  snapReach,
  SNAP_DISTANCE,
  SNAP_TARGETS,
  type SnapInput,
} from './snapping'

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
          anchor: 'Origin',
        },
        scope: 'layer',
        index: 0,
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
      anchor: 'ANCHOR',
      grid: 'GRID',
      component: 'COMPONENT',
      background: 'BACKGROUND',
      zone: 'ZONE',
      advance: 'WIDTH',
      baseline: 'baseline',
      xHeight: 'x-height',
      capHeight: 'cap height',
      ascender: 'ascender',
      descender: 'descender',
    },
    componentPoints: [],
    settings: { ...DEFAULT_SNAP_SETTINGS, nodes: true },
    viewport,
    exclude: new Set(['0:2']),
    ...overrides,
  }
}

describe('snapDelta', () => {
  it('switches handles and anchors independently from outline nodes', () => {
    const outline = buildEditOutline([
      makeContour([makePoint(180, 180), makePoint(230, 230, 'OffCurve')]),
    ])
    const settings = { ...DEFAULT_SNAP_SETTINGS, nodes: false, metrics: false }
    const handle = { x: 232, y: 232 }
    expect(
      snapDelta(input({ outline, settings }), { x: 0, y: 0 }, [handle]).label,
    ).toBe('HANDLE')
    expect(
      snapDelta(
        input({ outline, settings: { ...settings, handles: false } }),
        { x: 0, y: 0 },
        [handle],
      ).point,
    ).toBeNull()
    expect(
      snapDelta(
        input({ settings: { ...settings, anchors: false } }),
        { x: 0, y: 0 },
        [{ x: 301, y: 302 }],
      ).point,
    ).toBeNull()
  })

  it('snaps independently to alignment-zone boundaries and background points', () => {
    const settings = { ...DEFAULT_SNAP_SETTINGS, nodes: false, metrics: false }
    const scene = input({
      settings,
      zoneLines: [511, 525],
      backgroundPoints: [{ x: 432, y: 333 }],
    })
    expect(
      snapDelta(scene, { x: 0, y: 0 }, [{ x: 350, y: 514 }]).point,
    ).toEqual({ x: 350, y: 511 })
    expect(
      snapDelta(scene, { x: 0, y: 0 }, [{ x: 434, y: 335 }]).point,
    ).toEqual({ x: 432, y: 333 })
    const disabled = {
      ...scene,
      settings: { ...settings, zones: false, background: false },
    }
    expect(
      snapDelta(disabled, { x: 0, y: 0 }, [{ x: 350, y: 514 }]).point,
    ).toBeNull()
    expect(
      snapDelta(disabled, { x: 0, y: 0 }, [{ x: 434, y: 335 }]).point,
    ).toBeNull()
  })

  it('uses the configured snap radius in CSS pixels at every zoom', () => {
    const scene = input({
      settings: { ...DEFAULT_SNAP_SETTINGS, snapDistance: 2 },
    })
    scene.viewport.scale = 2
    expect(
      snapDelta(scene, { x: 0, y: 0 }, [{ x: 301, y: 301 }]).point,
    ).toBeNull()
    scene.settings.snapDistance = 4
    expect(
      snapDelta(scene, { x: 0, y: 0 }, [{ x: 301, y: 301 }]).point,
    ).toEqual({ x: 300, y: 300 })
    scene.settings.snapDistance = 0
    expect(
      snapDelta(scene, { x: 0, y: 0 }, [{ x: 300, y: 300 }]).point,
    ).toBeNull()
  })

  it('projects onto oblique guides without snapping along their length', () => {
    const scene = input()
    scene.guides = [
      {
        ...scene.guides[0]!,
        guide: {
          ...scene.guides[0]!.guide,
          x: 100,
          y: 100,
          angle: 45,
          name: 'diagonal',
        },
      },
    ]
    const match = snapDelta(scene, { x: 20, y: 30 }, [{ x: 220, y: 224 }])
    expect(match.point?.x).toBeCloseTo(222)
    expect(match.point?.y).toBeCloseTo(222)
    expect(match.delta.x).toBeCloseTo(22)
    expect(match.delta.y).toBeCloseTo(28)
    expect(match.label).toBe('diagonal')
  })

  it('snaps to horizontal advance boundaries when metrics are enabled', () => {
    const match = snapDelta(input(), { x: 0, y: 0 }, [{ x: 598, y: 333 }])
    expect(match.point).toEqual({ x: 600, y: 333 })
  })

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
            anchor: 'Origin',
          },
          scope: 'layer',
          index: 0,
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
  it('snaps to a component ghost node when the pref is on', () => {
    const match = snapDelta(
      input({ componentPoints: [{ x: 400, y: 400 }] }),
      { x: 0, y: 0 },
      [{ x: 402, y: 398 }],
    )
    expect(match.point).toEqual({ x: 400, y: 400 })
    expect(match.label).toBe('COMPONENT')
  })

  it('ignores component ghosts when the pref is off', () => {
    const match = snapDelta(
      input({
        componentPoints: [{ x: 400, y: 400 }],
        settings: { ...DEFAULT_SNAP_SETTINGS, components: false },
      }),
      { x: 0, y: 0 },
      [{ x: 402, y: 398 }],
    )
    expect(match.point).toBeNull()
  })

  it('snaps each axis to the grid when the grid is on', () => {
    const match = snapDelta(
      input({
        settings: {
          ...DEFAULT_SNAP_SETTINGS,
          nodes: false,
          metrics: false,
          grid: true,
          gridSize: 50,
        },
      }),
      { x: 0, y: 0 },
      [{ x: 302, y: 348 }],
    )
    expect(match.delta.x).toBeCloseTo(-2, 6)
    expect(match.delta.y).toBeCloseTo(2, 6)
    expect(match.label).toBe('GRID')
  })

  it('leaves the grid alone when its step is zero', () => {
    const match = snapDelta(
      input({
        settings: {
          ...DEFAULT_SNAP_SETTINGS,
          nodes: false,
          metrics: false,
          grid: true,
          gridSize: 0,
        },
      }),
      { x: 0, y: 0 },
      [{ x: 302, y: 348 }],
    )
    expect(match.point).toBeNull()
  })

  it('resolves node > handle > anchor > guide > metric > grid > alignment', () => {
    // Everything is within reach of (100, 500) at once; the winner names the
    // family that outranks the rest.
    const settings = {
      ...DEFAULT_SNAP_SETTINGS,
      grid: true,
      gridSize: 100,
    }
    const near = { x: 300, y: 252 }
    const guided = input({ settings })
    // Guide at y = 250 beats the metric lines and the grid.
    expect(snapDelta(guided, { x: 0, y: 0 }, [near]).label).toBe('mid')
    // With guides off the metric line at 250 is out of reach, so the grid at
    // y = 300 is still too far and only the grid's x = 300 matches.
    const noGuides = input({
      settings: { ...settings, guides: false },
    })
    const gridOnly = snapDelta(noGuides, { x: 0, y: 0 }, [near])
    expect(gridOnly.label).toBe('GRID')
    expect(gridOnly.delta.x).toBeCloseTo(0, 6)
    // A node beats every axis target, both axes at once.
    const nodeWins = snapDelta(guided, { x: 0, y: 0 }, [{ x: 98, y: 2 }])
    expect(nodeWins.label).toBe('NODE')
    expect(nodeWins.point).toEqual({ x: 100, y: 0 })
  })

  it('ignores nodes and alignment lines when the node pref is off', () => {
    const off = input({
      settings: { ...DEFAULT_SNAP_SETTINGS, nodes: false },
    })
    const match = snapDelta(off, { x: 0, y: 0 }, [{ x: 98, y: 320 }])
    expect(match.alignedX).toBeNull()
    expect(match.point).toBeNull()
  })
})

describe('per-target snap distances', () => {
  it('reaches exactly as far as each target is configured to', () => {
    // Only the two point families are live, so nothing else can answer.
    const settings = {
      ...DEFAULT_SNAP_SETTINGS,
      metrics: false,
      guides: false,
      zones: false,
      snapDistance: 2,
      distances: { ...NO_SNAP_DISTANCES, nodes: 20 },
    }
    // The node at (100, 0) is 12 units away: past the shared 2 px distance and
    // well inside the node target's own 20 px.
    const far = snapDelta(input({ settings }), { x: 0, y: 0 }, [
      { x: 112, y: 0 },
    ])
    expect(far.label).toBe('NODE')
    expect(far.delta).toEqual({ x: -12, y: 0 })
    // A handle keeps the shared distance, so the same gap does not snap it.
    const handles = {
      ...settings,
      distances: { ...NO_SNAP_DISTANCES, handles: 20 },
    }
    expect(
      snapDelta(input({ settings: handles }), { x: 0, y: 0 }, [
        { x: 112, y: 0 },
      ]).label,
    ).toBeNull()
  })

  it('lets one target be tightened without touching the others', () => {
    const settings = {
      ...DEFAULT_SNAP_SETTINGS,
      grid: true,
      gridSize: 50,
      distances: { ...NO_SNAP_DISTANCES, grid: 1 },
    }
    // 3 units off a grid line: inside the shared 6 px reach, outside the grid's
    // own 1 px, and the guide at y 250 is still free to catch it.
    const tight = snapDelta(input({ settings }), { x: 0, y: 0 }, [
      { x: 303, y: 403 },
    ])
    expect(tight.label).toBeNull()
    expect(
      snapDelta(
        input({
          settings: { ...DEFAULT_SNAP_SETTINGS, grid: true, gridSize: 50 },
        }),
        { x: 0, y: 0 },
        [{ x: 303, y: 403 }],
      ).label,
    ).toBe('GRID')
  })

  it('snaps nothing when every target is switched off or has no reach', () => {
    const off = Object.fromEntries(
      SNAP_TARGETS.map((target) => [target, false]),
    ) as Record<(typeof SNAP_TARGETS)[number], boolean>
    expect(
      snapDelta(
        input({ settings: { ...DEFAULT_SNAP_SETTINGS, ...off } }),
        { x: 0, y: 0 },
        [{ x: 100, y: 0 }],
      ),
    ).toMatchObject({ label: null, delta: { x: 0, y: 0 } })
    expect(
      snapDelta(
        input({
          settings: { ...DEFAULT_SNAP_SETTINGS, snapDistance: 0 },
        }),
        { x: 0, y: 0 },
        [{ x: 100, y: 0 }],
      ),
    ).toMatchObject({ label: null })
  })

  it('reports each target reach in font units at the current zoom', () => {
    const settings = {
      ...DEFAULT_SNAP_SETTINGS,
      snapDistance: 6,
      distances: { ...NO_SNAP_DISTANCES, nodes: 12, guides: -4 },
    }
    expect(snapReach(settings, 'nodes', 2)).toBe(6)
    expect(snapReach(settings, 'handles', 2)).toBe(3)
    // A negative or non-finite entry is not a distance; the shared one applies.
    expect(snapReach(settings, 'guides', 1)).toBe(6)
    expect(snapReach({ ...settings, grid: false }, 'grid', 1)).toBe(0)
  })
})
