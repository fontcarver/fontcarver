import { expect, it } from 'vitest'

import type { Contour, Guide } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import type { ScopedGuide } from '../hitTest'
import { buildEditOutline } from '../outlineGeometry'
import { NO_SNAP } from '../snapping'

import { MagnetTool, caughtNodes, guideOffset } from './magnet'
import {
  NO_TOOL_OVERLAY,
  type ToolContext,
  type ToolPointer,
} from './toolTypes'

function viewport(): Viewport {
  const view = new Viewport()
  view.setSize(400, 400, 1)
  view.scale = 1
  view.offset = { x: 0, y: 400 }
  return view
}

const contours: Contour[] = [
  {
    closed: true,
    name: null,
    id: null,
    points: [
      {
        x: 0,
        y: 100,
        kind: 'Line',
        smooth: false,
        name: null,
        tt_index: 'Unnumbered',
      },
      {
        x: 200,
        y: 104,
        kind: 'Line',
        smooth: false,
        name: null,
        tt_index: 'Unnumbered',
      },
      {
        x: 200,
        y: 400,
        kind: 'Line',
        smooth: false,
        name: null,
        tt_index: 'Unnumbered',
      },
      {
        x: 0,
        y: 400,
        kind: 'Line',
        smooth: false,
        name: null,
        tt_index: 'Unnumbered',
      },
    ],
  },
]

const layerGuide: Guide = {
  x: 0,
  y: 100,
  angle: 0,
  name: 'baseline helper',
  locked: false,
  color: null,
  id: 'g1',
  anchor: 'Origin',
}

function context(
  overrides: Partial<ToolContext> = {},
  guides: ScopedGuide[] = [{ guide: layerGuide, scope: 'layer', index: 0 }],
): ToolContext {
  const view = viewport()
  return {
    target: { doc: 1, glyph: 'A', layer: 'm1' },
    contours,
    outline: buildEditOutline(contours),
    anchors: [],
    components: [],
    guides,
    viewport: view,
    selection: new Map(),
    curves: 'cubic',
    labels: {} as ToolContext['labels'],
    snap: (point: { x: number; y: number }) => ({ point, match: NO_SNAP }),
    hitTest: () => null,
    segmentShortcut: () => null,
    after: undefined,
    onError: () => undefined,
    ...overrides,
  }
}

function pointer(x: number, y: number, alt = false): ToolPointer {
  const view = viewport()
  return {
    screen: { x, y },
    font: view.toFont({ x, y }),
    button: 0,
    modifiers: { shift: false, alt, mod: false, ctrl: false },
  }
}

const guideHit = {
  target: { kind: 'guide' as const, index: 0, scope: 'layer' as const },
  point: { x: 0, y: 100 },
  distance: 0,
}

it('measures the signed normal distance from a guide', () => {
  expect(guideOffset(layerGuide, { x: 50, y: 110 })).toBeCloseTo(10)
  expect(guideOffset(layerGuide, { x: 50, y: 90 })).toBeCloseTo(-10)
  const slanted = { ...layerGuide, angle: 90 }
  expect(guideOffset(slanted, { x: 20, y: 100 })).toBeCloseTo(-20)
})

it('catches on-curve nodes inside the radius and no handles', () => {
  const outline = buildEditOutline([
    {
      ...contours[0]!,
      points: [
        ...contours[0]!.points,
        {
          x: 10,
          y: 101,
          kind: 'OffCurve',
          smooth: false,
          name: null,
          tt_index: 'Unnumbered',
        },
      ],
    },
  ])
  const caught = caughtNodes(layerGuide, outline.points, 6)
  expect(caught).toEqual([
    { contour: 0, point: 0 },
    { contour: 0, point: 1 },
  ])
  expect(caughtNodes(layerGuide, outline.points, 2)).toEqual([
    { contour: 0, point: 0 },
  ])
})

it('counts the caught nodes in the singular and the plural', () => {
  const tool = new MagnetTool()
  const ctx = context({ hitTest: () => guideHit })
  tool.pointerDown(pointer(0, 300), ctx)
  tool.pointerMove(pointer(0, 296), ctx)
  expect(tool.overlay(ctx).hud).toBe(
    'Magnet: 2 nodes will follow this guide. Alt projects them onto the guide.',
  )
  // A radius that reaches one node reads as one node, not "1 nodes".
  const narrow = context({
    hitTest: () => guideHit,
    viewport: Object.assign(viewport(), { scale: 6 }),
  })
  tool.pointerMove(pointer(0, 300), narrow)
  expect(tool.overlay(narrow).hud).toBe(
    'Magnet: 1 node will follow this guide. Alt projects it onto the guide.',
  )
  tool.finish()
})

it('moves a dragged layer guide and links the nodes it caught in one entry', () => {
  const tool = new MagnetTool()
  const ctx = context({ hitTest: () => guideHit })
  expect(tool.pointerDown(pointer(0, 300), ctx)).toMatchObject({
    handled: true,
    editing: true,
  })
  expect(tool.busy).toBe(true)
  // Drag the guide four units up, onto the second node at y = 104.
  tool.pointerMove(pointer(0, 296), ctx)
  const overlay = tool.overlay(ctx)
  expect(overlay.marks).toHaveLength(2)
  expect(overlay.lines[0]?.dashed).toBe(true)
  const outcome = tool.pointerUp(pointer(0, 296), ctx)
  expect(outcome.editing).toBe(false)
  expect(outcome.apply?.command).toEqual({
    Batch: [
      {
        SetGuide: {
          glyph: 'A',
          layer: 'm1',
          master: null,
          index: 0,
          guide: { ...layerGuide, x: 0, y: 104 },
        },
      },
      {
        NodeLinks: {
          LinkToGuide: {
            glyph: 'A',
            layer: 'm1',
            guide_index: 0,
            points: [
              { contour: 0, point: 0 },
              { contour: 0, point: 1 },
            ],
          },
        },
      },
    ],
  })
  expect(tool.busy).toBe(false)
})

it('projects the caught nodes first when Alt is held', () => {
  const tool = new MagnetTool()
  const ctx = context({ hitTest: () => guideHit })
  tool.pointerDown(pointer(0, 300, true), ctx)
  tool.pointerMove(pointer(0, 296, true), ctx)
  expect(tool.overlay(ctx).hud).toContain('projected')
  const command = tool.pointerUp(pointer(0, 296, true), ctx).apply?.command as {
    Batch: Record<string, unknown>[]
  }
  expect(Object.keys(command.Batch[1] ?? {})).toEqual(['NodeLinks'])
  expect(command.Batch[1]).toMatchObject({
    NodeLinks: { ProjectToGuide: { guide_index: 0 } },
  })
  expect(command.Batch[2]).toMatchObject({
    NodeLinks: { LinkToGuide: { guide_index: 0 } },
  })
})

it('links without moving when the guide is only clicked', () => {
  const tool = new MagnetTool()
  const ctx = context({ hitTest: () => guideHit })
  tool.pointerDown(pointer(0, 300), ctx)
  const outcome = tool.pointerUp(pointer(0, 300), ctx)
  expect(outcome.apply?.command).toMatchObject({
    NodeLinks: { LinkToGuide: { guide_index: 0 } },
  })
})

it('refuses a master guide, a locked guide and empty canvas with a reason', () => {
  const tool = new MagnetTool()
  const master = context(
    {
      hitTest: () => ({
        ...guideHit,
        target: { kind: 'guide', index: 0, scope: 'master' },
      }),
    },
    [{ guide: layerGuide, scope: 'master', index: 0 }],
  )
  expect(tool.pointerDown(pointer(0, 300), master)).toEqual({ handled: true })
  expect(tool.busy).toBe(false)
  expect(tool.overlay(master).hud).toContain('Only a layer guide')

  const locked = context({ hitTest: () => guideHit }, [
    { guide: { ...layerGuide, locked: true }, scope: 'layer', index: 0 },
  ])
  tool.pointerDown(pointer(0, 300), locked)
  expect(tool.busy).toBe(false)
  expect(tool.overlay(locked).hud).toContain('locked')

  const empty = context()
  tool.pointerDown(pointer(300, 10), empty)
  expect(tool.busy).toBe(false)
  expect(tool.overlay(empty).hud).toContain('Press a layer guide')
})

it('skips nodes the lock policy refuses and abandons the drag on Escape', () => {
  const tool = new MagnetTool()
  const ctx = context({
    hitTest: () => guideHit,
    canSelect: (target) => target.kind !== 'node' || target.point !== 1,
  })
  tool.pointerDown(pointer(0, 300), ctx)
  tool.pointerMove(pointer(0, 296), ctx)
  const command = tool.pointerUp(pointer(0, 296), ctx).apply?.command as {
    Batch: Record<string, { LinkToGuide: { points: unknown[] } }>[]
  }
  expect(command.Batch[1]?.NodeLinks?.LinkToGuide.points).toEqual([
    { contour: 0, point: 0 },
  ])

  tool.pointerDown(pointer(0, 300), ctx)
  tool.pointerMove(pointer(0, 296), ctx)
  expect(tool.claimsKey('Escape')).toBe(true)
  expect(
    tool.keyDown({
      key: 'Escape',
      modifiers: pointer(0, 0).modifiers,
      repeat: false,
    }),
  ).toEqual({ handled: true, editing: false })
  expect(tool.busy).toBe(false)
  expect(tool.overlay(ctx)).toEqual(NO_TOOL_OVERLAY)
  expect(tool.pointerUp(pointer(0, 296), ctx)).toEqual({})
})

it('shows a grab cursor only over a layer guide', () => {
  const tool = new MagnetTool()
  const ctx = context({ hitTest: () => guideHit })
  expect(tool.cursor({ screen: { x: 0, y: 300 } }, ctx)).toBe('grab')
  expect(tool.cursor(null, ctx)).toBe('crosshair')
  expect(tool.cursor({ screen: { x: 0, y: 0 } }, context())).toBe('crosshair')
  tool.pointerDown(pointer(0, 300), ctx)
  expect(tool.cursor({ screen: { x: 0, y: 300 } }, ctx)).toBe('grabbing')
})
