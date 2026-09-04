import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type Command, type EditApplyParams } from '../../api'
import { makeContour, makePoint } from '../../test/fontFixtures'
import { createMockTransport, type MockTransport } from '../../test/mockTransport'
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

describe('constrainDelta', () => {
  it('locks to the nearest 0/45/90 degree direction', () => {
    expect(constrainDelta({ x: 40, y: 3 })).toEqual({ x: 40.11234224026316, y: 0 })
    expect(constrainDelta({ x: 3, y: 40 })).toEqual({ x: 0, y: 40.11234224026316 })
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
    expect(command && 'MovePoints' in command && command.MovePoints.delta[1]).toBe(0)
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
