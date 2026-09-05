import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../../test/fontFixtures'
import { click, makeToolContext, toolPointer } from '../../test/toolContext'
import { NO_MODIFIERS } from '../gestures/drag'

import { KnifeTool } from './knife'
import { MeasureTool, measureSpans } from './measure'
import { PencilTool, polylineContour } from './pencil'
import { ShapeTool, shapeBox } from './shapes'

const shift = { ...NO_MODIFIERS, shift: true }
const alt = { ...NO_MODIFIERS, alt: true }

const square = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(200, 200, 'Line'),
  makePoint(0, 200, 'Line'),
])

interface BatchCommand {
  Batch: [
    { AddContour: { contour: { points: { x: number; y: number }[] } } },
    { PathOp: { op: { Simplify: { error: number } } } },
  ]
}

describe('PencilTool', () => {
  it('commits a polyline and a simplify pass as one undo entry', () => {
    const context = makeToolContext()
    const pencil = new PencilTool()
    pencil.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    for (let step = 1; step <= 5; step += 1)
      pencil.pointerMove(
        toolPointer(context, { x: step * 20, y: step * 5 }),
        context,
      )
    const outcome = pencil.pointerUp(
      toolPointer(context, { x: 100, y: 25 }),
      context,
    ) as { apply?: { label: string; command: BatchCommand } }

    expect(outcome.apply?.label).toBe('Draw')
    const batch = outcome.apply?.command.Batch
    expect(batch?.[0].AddContour.contour.points).toHaveLength(6)
    expect(batch?.[1].PathOp.op.Simplify.error).toBe(1)
  })

  it('drops samples closer together than two pixels', () => {
    const context = makeToolContext()
    const pencil = new PencilTool()
    pencil.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    pencil.pointerMove(toolPointer(context, { x: 1, y: 0 }), context)
    pencil.pointerMove(toolPointer(context, { x: 1.5, y: 0 }), context)
    pencil.pointerMove(toolPointer(context, { x: 40, y: 0 }), context)
    const outcome = pencil.pointerUp(
      toolPointer(context, { x: 40, y: 0 }),
      context,
    ) as { apply?: { command: BatchCommand } }

    expect(
      outcome.apply?.command.Batch[0].AddContour.contour.points.map(
        (point) => point.x,
      ),
    ).toEqual([0, 40])
  })

  it('keeps the span straight while Shift is held', () => {
    const context = makeToolContext()
    const pencil = new PencilTool()
    pencil.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    pencil.pointerMove(toolPointer(context, { x: 40, y: 10 }, shift), context)
    pencil.pointerMove(toolPointer(context, { x: 80, y: 20 }, shift), context)
    pencil.pointerMove(toolPointer(context, { x: 120, y: 30 }, shift), context)
    const outcome = pencil.pointerUp(
      toolPointer(context, { x: 120, y: 30 }, shift),
      context,
    ) as { apply?: { command: BatchCommand } }

    // One straight span: the free end was replaced, never appended to.
    expect(
      outcome.apply?.command.Batch[0].AddContour.contour.points.map(
        (point) => point.x,
      ),
    ).toEqual([0, 120])
  })

  it('closes a stroke that comes back to where it started', () => {
    const context = makeToolContext()
    const pencil = new PencilTool()
    pencil.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    for (const at of [
      { x: 40, y: 0 },
      { x: 40, y: 40 },
      { x: 0, y: 40 },
      { x: 1, y: 1 },
    ])
      pencil.pointerMove(toolPointer(context, at), context)
    const outcome = pencil.pointerUp(
      toolPointer(context, { x: 1, y: 1 }),
      context,
    ) as { apply?: { command: BatchCommand } }
    const contour = outcome.apply?.command.Batch[0].AddContour.contour as {
      closed: boolean
      points: unknown[]
    }
    expect(contour.closed).toBe(true)
    expect(contour.points).toHaveLength(4)
  })

  it('sends nothing for a stroke of one sample', () => {
    const context = makeToolContext()
    const pencil = new PencilTool()
    expect(
      click(pencil, context, { x: 10, y: 10 }) as { apply?: unknown },
    ).toEqual({ handled: true })
    expect(polylineContour([{ x: 0, y: 0 }], false)).toBeNull()
  })
})

describe('KnifeTool', () => {
  it('replaces a cut rectangle with its two halves in one batch', () => {
    const context = makeToolContext({ contours: [square] })
    const knife = new KnifeTool()
    knife.pointerDown(toolPointer(context, { x: 100, y: -50 }), context)
    knife.pointerMove(toolPointer(context, { x: 100, y: 250 }), context)
    const outcome = knife.pointerUp(
      toolPointer(context, { x: 100, y: 250 }),
      context,
    ) as {
      apply?: {
        label: string
        command: { Batch: { DeleteContours?: { contours: number[] } }[] }
      }
    }

    expect(outcome.apply?.label).toBe('Cut')
    const batch = outcome.apply?.command.Batch ?? []
    expect(batch).toHaveLength(3)
    expect(batch[0]?.DeleteContours?.contours).toEqual([0])
  })

  it('sends nothing when the stroke misses every contour', () => {
    const context = makeToolContext({ contours: [square] })
    const knife = new KnifeTool()
    knife.pointerDown(toolPointer(context, { x: 400, y: 0 }), context)
    knife.pointerMove(toolPointer(context, { x: 400, y: 200 }), context)
    const outcome = knife.pointerUp(
      toolPointer(context, { x: 400, y: 200 }),
      context,
    )
    expect(outcome.apply).toBeUndefined()
  })

  it('marks the crossings while the stroke is being drawn', () => {
    const context = makeToolContext({ contours: [square] })
    const knife = new KnifeTool()
    knife.pointerDown(toolPointer(context, { x: 100, y: -50 }), context)
    knife.pointerMove(toolPointer(context, { x: 100, y: 250 }), context)
    const overlay = knife.overlay(context)
    expect(overlay.lines).toHaveLength(1)
    expect(overlay.marks).toHaveLength(2)
  })

  it('constrains the stroke to 45° with Shift', () => {
    const context = makeToolContext({ contours: [square] })
    const knife = new KnifeTool()
    knife.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    knife.pointerMove(toolPointer(context, { x: 100, y: 10 }, shift), context)
    // The direction snaps to the nearest 45°, keeping the drag's length, the
    // same rule `constrainDelta` applies to a Select drag.
    const end = knife.overlay(context).lines[0]?.to
    expect(end?.y).toBe(0)
    expect(end?.x).toBeCloseTo(Math.hypot(100, 10), 6)
  })
})

describe('ShapeTool', () => {
  it('commits a rectangle for the dragged box', () => {
    const context = makeToolContext()
    const tool = new ShapeTool('rectangle')
    tool.pointerDown(toolPointer(context, { x: 10, y: 20 }), context)
    tool.pointerMove(toolPointer(context, { x: 110, y: 220 }), context)
    const outcome = tool.pointerUp(
      toolPointer(context, { x: 110, y: 220 }),
      context,
    ) as {
      apply?: {
        label: string
        command: {
          AddContour: { contour: { points: { x: number; y: number }[] } }
        }
      }
    }

    expect(outcome.apply?.label).toBe('Rectangle')
    expect(
      outcome.apply?.command.AddContour.contour.points.map((point) => [
        point.x,
        point.y,
      ]),
    ).toEqual([
      [10, 20],
      [110, 20],
      [110, 220],
      [10, 220],
    ])
  })

  it('opens the numeric dialog for a click that never dragged', () => {
    const context = makeToolContext()
    const tool = new ShapeTool('ellipse')
    const outcome = click(tool, context, { x: 40, y: 60 }) as {
      dialog?: { kind: string; at: { x: number; y: number } }
    }
    expect(outcome.dialog).toEqual({
      kind: 'ellipse',
      at: { x: 40, y: 60 },
    })
  })

  it('makes a square with Shift and grows from the centre with Alt', () => {
    expect(shapeBox({ x: 0, y: 0 }, { x: 100, y: 30 }, shift)).toEqual({
      x0: 0,
      y0: 0,
      x1: 100,
      y1: 100,
    })
    expect(shapeBox({ x: 50, y: 50 }, { x: 90, y: 70 }, alt)).toEqual({
      x0: 10,
      y0: 30,
      x1: 90,
      y1: 70,
    })
  })

  it('draws four cubic quarters on a cubic layer', () => {
    const context = makeToolContext({ curves: 'cubic' })
    const tool = new ShapeTool('ellipse')
    tool.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    tool.pointerMove(toolPointer(context, { x: 200, y: 100 }), context)
    const outcome = tool.pointerUp(
      toolPointer(context, { x: 200, y: 100 }),
      context,
    ) as {
      apply?: {
        command: { AddContour: { contour: { points: { kind: string }[] } } }
      }
    }
    const kinds =
      outcome.apply?.command.AddContour.contour.points.map(
        (point) => point.kind,
      ) ?? []
    expect(kinds.filter((kind) => kind === 'Curve')).toHaveLength(4)
    expect(kinds.filter((kind) => kind === 'OffCurve')).toHaveLength(8)
  })

  it('draws eight quadratic eighths on a quadratic layer', () => {
    const context = makeToolContext({ curves: 'quadratic' })
    const tool = new ShapeTool('ellipse')
    tool.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    tool.pointerMove(toolPointer(context, { x: 200, y: 200 }), context)
    const outcome = tool.pointerUp(
      toolPointer(context, { x: 200, y: 200 }),
      context,
    ) as {
      apply?: {
        command: { AddContour: { contour: { points: { kind: string }[] } } }
      }
    }
    const kinds =
      outcome.apply?.command.AddContour.contour.points.map(
        (point) => point.kind,
      ) ?? []
    expect(kinds.filter((kind) => kind === 'QCurve')).toHaveLength(8)
    expect(kinds.filter((kind) => kind === 'OffCurve')).toHaveLength(8)
  })

  it('reads out the size while dragging', () => {
    const context = makeToolContext()
    const tool = new ShapeTool('rectangle')
    tool.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    tool.pointerMove(toolPointer(context, { x: 312, y: 84 }), context)
    expect(tool.overlay(context).hud).toBe('w 312 × h 84')
  })
})

describe('MeasureTool', () => {
  it('reads out the distance and the angle, and marks the crossings', () => {
    const context = makeToolContext({ contours: [square] })
    const tool = new MeasureTool()
    tool.pointerDown(toolPointer(context, { x: -50, y: 100 }), context)
    tool.pointerMove(toolPointer(context, { x: 250, y: 100 }), context)
    const overlay = tool.overlay(context)

    expect(overlay.hud).toBe('d 300  ∠ 0.0°')
    expect(overlay.marks.map((mark) => mark.x)).toEqual([0, 200])
    expect(overlay.labels).toEqual([{ at: { x: 100, y: 100 }, text: '200' }])
  })

  it('emits no command at all', () => {
    const context = makeToolContext({ contours: [square] })
    const tool = new MeasureTool()
    tool.pointerDown(toolPointer(context, { x: -50, y: 100 }), context)
    tool.pointerMove(toolPointer(context, { x: 250, y: 100 }), context)
    expect(
      tool.pointerUp(toolPointer(context, { x: 250, y: 100 }), context).apply,
    ).toBeUndefined()
  })

  it('labels each span between consecutive crossings', () => {
    expect(
      measureSpans([
        { x: 0, y: 0 },
        { x: 30, y: 0 },
        { x: 100, y: 0 },
      ]),
    ).toEqual([
      { at: { x: 15, y: 0 }, text: '30' },
      { at: { x: 65, y: 0 }, text: '70' },
    ])
  })
})
