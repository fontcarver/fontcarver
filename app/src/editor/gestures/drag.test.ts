import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type Command, type EditApplyParams } from '../../api'
import { makeContour, makePoint } from '../../test/fontFixtures'
import {
  createMockTransport,
  type MockTransport,
} from '../../test/mockTransport'
import { immediateScheduler } from '../editSession'
import type { Hit } from '../hitTest'
import { EMPTY_SELECTION, selectionOf } from '../selection'

import {
  beginDrag,
  constrainDelta,
  NO_MODIFIERS,
  planDrag,
  type DragContext,
} from './drag'

// A square whose top-right corner carries one off-curve handle.
const contour = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(200, 200, 'Line', true),
  makePoint(140, 260, 'OffCurve'),
  makePoint(0, 200, 'Curve'),
])

const labels = {
  movePoints: (count: number) => `Move ${count} points`,
  moveHandles: (count: number) => `Move ${count} handles`,
  moveHandle: 'Move handle',
  reshapeSegment: 'Reshape segment',
  moveAnchor: 'Move anchor',
  moveComponent: 'Move component',
}

function transport(): MockTransport {
  const mock = createMockTransport({
    'doc.edit_begin': () => 7,
    'doc.edit_apply': () => ({
      glyphs: [42],
      font_info: false,
      masters: false,
      glyph_order: false,
      metrics: false,
      layers: false,
      guides: false,
    }),
    'doc.edit_end': () => null,
    'doc.undo': () => null,
  })
  setTransport(mock)
  return mock
}

function context(overrides: Partial<DragContext> = {}): DragContext {
  return {
    doc: 1,
    glyph: 'a',
    layer: 'layer-1',
    contours: [contour],
    anchors: [],
    selection: EMPTY_SELECTION,
    labels,
    scheduler: immediateScheduler,
    ...overrides,
  }
}

const nodeHit: Hit = {
  target: { kind: 'node', contour: 0, point: 1 },
  point: { x: 200, y: 0 },
  distance: 0,
}

const handleHit: Hit = {
  target: { kind: 'handle', contour: 0, point: 3 },
  point: { x: 140, y: 260 },
  distance: 0,
}

function applied(mock: MockTransport): Command[] {
  return mock.calls
    .filter((entry) => entry.method === 'doc.edit_apply')
    .map((entry) => (entry.params as EditApplyParams).command)
}

afterEach(() => setTransport(undefined))

describe('planDrag', () => {
  it('moves the whole selection when the target is part of it', () => {
    const selection = selectionOf([
      { kind: 'node', contour: 0, point: 1 },
      { kind: 'node', contour: 0, point: 2 },
      { kind: 'node', contour: 0, point: 4 },
    ])
    const plan = planDrag(context({ selection }), nodeHit, NO_MODIFIERS)
    expect(plan?.refs).toHaveLength(3)
    expect(plan?.label).toBe('Move 3 points')
    expect(plan?.handleMode).toBe('WithHandles')
  })

  it('moves only the target when it is outside the selection', () => {
    const selection = selectionOf([{ kind: 'node', contour: 0, point: 4 }])
    const plan = planDrag(context({ selection }), nodeHit, NO_MODIFIERS)
    expect(plan?.refs).toEqual([{ contour: 0, point: 1 }])
    expect(plan?.label).toBe('Move 1 points')
  })

  it('names a handles-only move for what it is', () => {
    // Counting nodes would label a marquee of handles "Move 0 points".
    const selection = selectionOf([
      { kind: 'handle', contour: 0, point: 3 },
      { kind: 'node', contour: 0, point: 1 },
    ])
    expect(
      planDrag(context({ selection }), handleHit, NO_MODIFIERS)?.label,
    ).toBe('Move handle')

    const handlesOnly = selectionOf([{ kind: 'handle', contour: 0, point: 3 }])
    const plan = planDrag(
      context({ selection: handlesOnly }),
      {
        target: { kind: 'node', contour: 0, point: 1 },
        point: { x: 200, y: 0 },
        distance: 0,
      },
      NO_MODIFIERS,
    )
    expect(plan?.label).toBe('Move 1 points')
  })

  it('leaves the handles behind while Alt is held', () => {
    const plan = planDrag(context(), nodeHit, {
      ...NO_MODIFIERS,
      alt: true,
    })
    expect(plan?.handleMode).toBe('PointsOnly')
  })

  it('names a handle drag and a segment drag', () => {
    expect(planDrag(context(), handleHit, NO_MODIFIERS)?.label).toBe(
      'Move handle',
    )
    const segment = planDrag(
      context(),
      {
        target: { kind: 'segment', contour: 0, segment: 2 },
        point: { x: 150, y: 240 },
        distance: 0,
      },
      NO_MODIFIERS,
    )
    expect(segment?.label).toBe('Reshape segment')
    // Segment 2 is the curve from (200, 200) to (0, 200): its one control moves.
    expect(segment?.refs).toEqual([{ contour: 0, point: 3 }])
  })
})

// A cubic whose two controls lean in opposite directions, so an angle-locked
// reshape has to give each of them its own delta.
const curved = makeContour([
  makePoint(0, 0),
  makePoint(60, 120, 'OffCurve'),
  makePoint(140, 120, 'OffCurve'),
  makePoint(200, 0, 'Curve'),
])

const segmentHit: Hit = {
  target: { kind: 'segment', contour: 0, segment: 0 },
  point: { x: 100, y: 90 },
  distance: 0,
}

describe('segment reshaping', () => {
  it('moves both controls together on a plain drag', async () => {
    const mock = transport()
    const drag = beginDrag(
      context({ contours: [curved] }),
      segmentHit,
      { x: 100, y: 90 },
      NO_MODIFIERS,
    )
    await drag?.ready()
    drag?.move({ x: 100, y: 110 }, NO_MODIFIERS)
    await drag?.finish()

    expect(applied(mock)).toEqual([
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: [
            { contour: 0, point: 1 },
            { contour: 0, point: 2 },
          ],
          delta: [0, 20],
          handles: 'WithHandles',
        },
      },
    ])
  })

  it('keeps each handle on its own line while Alt is held', async () => {
    const mock = transport()
    const drag = beginDrag(
      context({ contours: [curved] }),
      segmentHit,
      { x: 100, y: 90 },
      { ...NO_MODIFIERS, alt: true },
    )
    await drag?.ready()
    drag?.move({ x: 100, y: 110 }, { ...NO_MODIFIERS, alt: true })
    await drag?.finish()

    // Each control is projected onto the line from its own node, so the two
    // handles keep their angles and the curve keeps its tangents.
    const command = applied(mock)[0]
    expect(command && 'Batch' in command ? command.Batch.length : 0).toBe(2)
    const steps =
      command && 'Batch' in command
        ? command.Batch.map((entry) =>
            'MovePoints' in entry
              ? [entry.MovePoints.points, entry.MovePoints.delta]
              : null,
          )
        : []
    expect(steps[0]?.[0]).toEqual([{ contour: 0, point: 1 }])
    expect((steps[0]?.[1] as number[])[0]).toBeCloseTo(8, 3)
    expect((steps[0]?.[1] as number[])[1]).toBeCloseTo(16, 3)
    expect(steps[1]?.[0]).toEqual([{ contour: 0, point: 2 }])
    expect((steps[1]?.[1] as number[])[0]).toBeCloseTo(-8, 3)
    expect((steps[1]?.[1] as number[])[1]).toBeCloseTo(16, 3)

    const preview = drag?.preview.contours[0]?.points
    expect(preview?.[1]?.x).toBeCloseTo(68, 3)
    expect(preview?.[2]?.x).toBeCloseTo(132, 3)
  })
})

describe('proportional nudge', () => {
  it("scales the neighbouring node's handle along the segment", async () => {
    const mock = transport()
    const nudge = { ...NO_MODIFIERS, mod: true, alt: true }
    const drag = beginDrag(
      context({ contours: [curved] }),
      {
        target: { kind: 'node', contour: 0, point: 3 },
        point: { x: 200, y: 0 },
        distance: 0,
      },
      { x: 200, y: 0 },
      nudge,
    )
    await drag?.ready()
    // The chord from (0, 0) grows from 200 to 300 units, so the handle that
    // belongs to the *other* end of the segment grows by the same half again
    // (design spec §4.3: Ctrl+Alt+drag nudges, adjacent unselected handles
    // scale proportionally).
    drag?.move({ x: 300, y: 0 }, nudge)
    await drag?.finish()

    const command = applied(mock)[0]
    const moves =
      command && 'Batch' in command
        ? command.Batch.map((entry) =>
            'MovePoints' in entry
              ? [entry.MovePoints.points[0]?.point, entry.MovePoints.delta]
              : null,
          )
        : []
    expect(moves).toEqual([
      [1, [30, 60]],
      [2, [100, 0]],
      [3, [100, 0]],
    ])

    const preview = drag?.preview.contours[0]?.points
    expect(preview?.[1]).toMatchObject({ x: 90, y: 180 })
    expect(preview?.[3]).toMatchObject({ x: 300, y: 0 })
  })

  it('leaves a handle alone when it is part of the selection', async () => {
    const mock = transport()
    const nudge = { ...NO_MODIFIERS, mod: true, alt: true }
    const selection = selectionOf([
      { kind: 'node', contour: 0, point: 3 },
      { kind: 'handle', contour: 0, point: 1 },
    ])
    const drag = beginDrag(
      context({ contours: [curved], selection }),
      {
        target: { kind: 'node', contour: 0, point: 3 },
        point: { x: 200, y: 0 },
        distance: 0,
      },
      { x: 200, y: 0 },
      nudge,
    )
    await drag?.ready()
    drag?.move({ x: 300, y: 0 }, nudge)
    await drag?.finish()

    // Point 1 travels with the selection, so it is not scaled on top of that:
    // one plain move of the whole set, and the handle lands 100 units right.
    expect(applied(mock)).toEqual([
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: [
            { contour: 0, point: 1 },
            { contour: 0, point: 3 },
          ],
          delta: [100, 0],
          handles: 'WithHandles',
        },
      },
    ])
    expect(drag?.preview.contours[0]?.points[1]).toMatchObject({
      x: 160,
      y: 120,
    })
  })
})

describe('constrainDelta', () => {
  it('locks to the nearest 0/45/90 degree direction', () => {
    expect(constrainDelta({ x: 40, y: 3 })).toEqual({
      x: 40.11234224026316,
      y: 0,
    })
    expect(constrainDelta({ x: 3, y: 40 })).toEqual({
      x: 0,
      y: 40.11234224026316,
    })
    const diagonal = constrainDelta({ x: 30, y: 28 })
    expect(diagonal.x).toBeCloseTo(diagonal.y, 6)
  })
})

describe('DragGesture', () => {
  it('is one edit_begin, one apply per frame and one edit_end', async () => {
    const mock = transport()
    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    expect(drag).not.toBeNull()
    if (!drag) return
    await drag.ready()

    drag.move({ x: 210, y: 0 }, NO_MODIFIERS)
    await drag.flush()
    drag.move({ x: 225, y: 0 }, NO_MODIFIERS)
    await drag.flush()
    expect(await drag.finish()).toBe(true)

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(mock.calls[0]?.params).toEqual({ doc: 1, label: 'Move 1 points' })
    // Each frame sends the step since the last one; the sum is the gesture.
    expect(applied(mock)).toEqual([
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: [{ contour: 0, point: 1 }],
          delta: [10, 0],
          handles: 'WithHandles',
        },
      },
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: [{ contour: 0, point: 1 }],
          delta: [15, 0],
          handles: 'WithHandles',
        },
      },
    ])
    expect(mock.calls[3]?.params).toEqual({ doc: 1, token: 7 })
  })

  it('sends nothing and records no undo entry when the pointer never moves', async () => {
    const mock = transport()
    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    await drag?.ready()
    drag?.move({ x: 200, y: 0 }, NO_MODIFIERS)
    expect(await drag?.finish()).toBe(false)

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_end',
    ])
  })

  it('coalesces several pointer moves inside one animation frame', async () => {
    const mock = transport()
    const frames: (() => void)[] = []
    const drag = beginDrag(
      context({
        scheduler: {
          request: (callback) => frames.push(callback),
          cancel: () => undefined,
        },
      }),
      nodeHit,
      { x: 200, y: 0 },
      NO_MODIFIERS,
    )
    await drag?.ready()
    drag?.move({ x: 205, y: 0 }, NO_MODIFIERS)
    drag?.move({ x: 212, y: 0 }, NO_MODIFIERS)
    drag?.move({ x: 230, y: 0 }, NO_MODIFIERS)
    expect(applied(mock)).toHaveLength(0)
    frames.forEach((frame) => frame())
    await drag?.finish()

    expect(applied(mock)).toEqual([
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: [{ contour: 0, point: 1 }],
          delta: [30, 0],
          handles: 'WithHandles',
        },
      },
    ])
  })

  it('previews the move locally before the kernel answers', () => {
    transport()
    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    drag?.move({ x: 220, y: 40 }, NO_MODIFIERS)
    const point = drag?.preview.contours[0]?.points[1]
    expect(point).toMatchObject({ x: 220, y: 40 })
    // The neighbouring handle travels with the node.
    expect(drag?.preview.contours[0]?.points[3]).toMatchObject({
      x: 140,
      y: 260,
    })
  })

  it('carries the adjacent handles of a moved node', () => {
    transport()
    const drag = beginDrag(
      context(),
      {
        target: { kind: 'node', contour: 0, point: 2 },
        point: { x: 200, y: 200 },
        distance: 0,
      },
      { x: 200, y: 200 },
      NO_MODIFIERS,
    )
    drag?.move({ x: 210, y: 200 }, NO_MODIFIERS)
    expect(drag?.preview.contours[0]?.points[3]).toMatchObject({
      x: 150,
      y: 260,
    })
  })

  it('constrains to one axis while Shift is held', async () => {
    const mock = transport()
    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    await drag?.ready()
    drag?.move({ x: 240, y: 6 }, { ...NO_MODIFIERS, shift: true })
    await drag?.finish()
    const command = applied(mock)[0]
    expect(
      command && 'MovePoints' in command && command.MovePoints.delta[1],
    ).toBe(0)
  })

  it('ends the gesture and undoes it when Escape cancels a drag', async () => {
    const mock = transport()
    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    await drag?.ready()
    drag?.move({ x: 260, y: 0 }, NO_MODIFIERS)
    await drag?.flush()
    await drag?.cancel()

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
    ])
  })

  it('undoes a cancelled gesture whose apply was still in flight', async () => {
    // Escape normally arrives while an apply is on the wire, because applies
    // are queued every animation frame for as long as the pointer moves.
    let release = () => undefined as void
    const inFlight = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    const mock = createMockTransport({
      'doc.edit_begin': () => 7,
      'doc.edit_apply': async () => {
        await inFlight
        return {
          glyphs: [42],
          font_info: false,
          masters: false,
          glyph_order: false,
          metrics: false,
          layers: false,
          guides: false,
        }
      },
      'doc.edit_end': () => null,
      'doc.undo': () => null,
    })
    setTransport(mock)

    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    await drag?.ready()
    drag?.move({ x: 260, y: 0 }, NO_MODIFIERS)
    const cancelled = drag?.cancel()
    release()
    await cancelled

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
    ])
  })

  it('reports a failing apply instead of leaving an unhandled rejection', async () => {
    const errors: unknown[] = []
    setTransport(
      createMockTransport({
        'doc.edit_begin': () => 7,
        'doc.edit_apply': () => {
          throw new Error('invalid_params: stale edit token')
        },
        'doc.edit_end': () => null,
      }),
    )
    const drag = beginDrag(
      { ...context(), onError: (error) => errors.push(error) },
      nodeHit,
      { x: 200, y: 0 },
      NO_MODIFIERS,
    )
    await drag?.ready()
    drag?.move({ x: 260, y: 0 }, NO_MODIFIERS)
    await drag?.flush()
    // The gesture still closes cleanly, and the failure is reported.
    expect(await drag?.finish()).toBe(false)
    expect(String(errors[0])).toContain('stale edit token')
  })

  it('does not undo a cancelled gesture that never applied anything', async () => {
    const mock = transport()
    const drag = beginDrag(context(), nodeHit, { x: 200, y: 0 }, NO_MODIFIERS)
    await drag?.ready()
    await drag?.cancel()
    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_end',
    ])
  })
})
