import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../../test/fontFixtures'
import { click, makeToolContext, toolPointer } from '../../test/toolContext'
import { NO_MODIFIERS } from '../gestures/drag'

import { PenTool } from './pen'

const shift = { ...NO_MODIFIERS, shift: true }
const alt = { ...NO_MODIFIERS, alt: true }

describe('PenTool', () => {
  it('closes a triangle with three clicks and a click on the first node', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    click(pen, context, { x: 200, y: 0 })
    click(pen, context, { x: 100, y: 180 })
    const outcome = click(pen, context, { x: 0, y: 0 }) as {
      apply?: { command: unknown; label: string }
    }

    expect(outcome.apply?.label).toBe('Draw path')
    const command = outcome.apply?.command as {
      AddContour: { contour: { closed: boolean; points: unknown[] } }
    }
    expect(command.AddContour.contour.closed).toBe(true)
    expect(command.AddContour.contour.points).toHaveLength(3)
    expect(pen.nodeCount).toBe(0)
  })

  it('emits nothing until the path is finished', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    expect(click(pen, context, { x: 0, y: 0 })).toEqual({ handled: true })
    expect(click(pen, context, { x: 200, y: 0 })).toEqual({ handled: true })
    expect(pen.nodeCount).toBe(2)
  })

  it('turns a press-drag into a mirrored pair of handles', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    pen.pointerDown(toolPointer(context, { x: 200, y: 0 }), context)
    pen.pointerMove(toolPointer(context, { x: 240, y: 40 }), context)
    pen.pointerUp(toolPointer(context, { x: 240, y: 40 }), context)
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        command: {
          AddContour: {
            contour: { points: { x: number; y: number; kind: string }[] }
          }
        }
      }
    }

    const points = outcome.apply?.command.AddContour.contour.points ?? []
    // Line start, the incoming handle mirrored through the node, the outgoing
    // handle the drag set, and the node itself.
    expect(points.map((point) => point.kind)).toEqual([
      'Move',
      'OffCurve',
      'OffCurve',
      'Curve',
    ])
    expect(points[1]).toMatchObject({ x: 0, y: 0 })
    expect(points[2]).toMatchObject({ x: 160, y: -40 })
    expect(points[3]).toMatchObject({ x: 200, y: 0, smooth: true })
  })

  it('keeps the incoming handle where it was when Alt is held', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    pen.pointerDown(toolPointer(context, { x: 200, y: 0 }), context)
    pen.pointerMove(toolPointer(context, { x: 240, y: 40 }, alt), context)
    pen.pointerUp(toolPointer(context, { x: 240, y: 40 }, alt), context)
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        command: {
          AddContour: {
            contour: { points: { kind: string; smooth: boolean }[] }
          }
        }
      }
    }

    // No incoming handle was mirrored, so the last span is still a line.
    expect(
      outcome.apply?.command.AddContour.contour.points.map(
        (point) => point.kind,
      ),
    ).toEqual(['Move', 'Line'])
  })

  it('constrains a handle drag to 45° with Shift', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    pen.pointerDown(toolPointer(context, { x: 200, y: 0 }), context)
    pen.pointerMove(toolPointer(context, { x: 260, y: 10 }, shift), context)
    const overlay = pen.overlay(context)
    const handle = overlay.lines.find((line) => line.to.x > 200)
    expect(handle?.to.y).toBeCloseTo(0, 6)
  })

  it('repositions the last node while Space is held', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    pen.pointerDown(toolPointer(context, { x: 200, y: 0 }), context)
    pen.setSpaceHeld(true)
    pen.pointerMove(toolPointer(context, { x: 220, y: 40 }), context)
    pen.setSpaceHeld(false)
    pen.pointerUp(toolPointer(context, { x: 220, y: 40 }), context)
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        command: {
          AddContour: { contour: { points: { x: number; y: number }[] } }
        }
      }
    }

    expect(outcome.apply?.command.AddContour.contour.points[1]).toMatchObject({
      x: 220,
      y: 40,
    })
  })

  it('extends an existing open contour as one batch', () => {
    const open = makeContour(
      [makePoint(0, 0, 'Move'), makePoint(100, 0, 'Line')],
      false,
    )
    const context = makeToolContext({ contours: [open] })
    const pen = new PenTool()
    // Pick the path up at its end node, then add one more point.
    click(pen, context, { x: 100, y: 0 })
    click(pen, context, { x: 200, y: 60 })
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        label: string
        command: {
          Batch: [{ DeleteContours: { contours: number[] } }, unknown]
        }
      }
    }

    expect(outcome.apply?.label).toBe('Extend path')
    expect(outcome.apply?.command.Batch[0].DeleteContours.contours).toEqual([0])
    const added = outcome.apply?.command.Batch[1] as {
      AddContour: { contour: { closed: boolean; points: { x: number }[] } }
    }
    expect(added.AddContour.contour.closed).toBe(false)
    expect(added.AddContour.contour.points.map((point) => point.x)).toEqual([
      0, 100, 200,
    ])
  })

  it('keeps the extended contour name and id', () => {
    const open = {
      ...makeContour(
        [makePoint(0, 0, 'Move'), makePoint(100, 0, 'Line')],
        false,
      ),
      name: 'stem',
      id: 'c-7',
    }
    const context = makeToolContext({ contours: [open] })
    const pen = new PenTool()
    click(pen, context, { x: 100, y: 0 })
    click(pen, context, { x: 200, y: 60 })
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        command: {
          Batch: [
            unknown,
            {
              AddContour: {
                contour: { name: string | null; id: string | null }
              }
            },
          ]
        }
      }
    }
    const added = outcome.apply?.command.Batch[1].AddContour.contour
    // The extended path *is* the contour that was picked up.
    expect(added?.name).toBe('stem')
    expect(added?.id).toBe('c-7')
  })

  it('gives a freshly drawn path no borrowed identity', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    click(pen, context, { x: 100, y: 0 })
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        command: { AddContour: { contour: { name: null; id: null } } }
      }
    }
    expect(outcome.apply?.command.AddContour.contour.name).toBeNull()
    expect(outcome.apply?.command.AddContour.contour.id).toBeNull()
  })

  it('reverses a path picked up at its first node', () => {
    const open = makeContour(
      [makePoint(0, 0, 'Move'), makePoint(100, 0, 'Line')],
      false,
    )
    const context = makeToolContext({ contours: [open] })
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    click(pen, context, { x: -100, y: 0 })
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: {
        command: {
          Batch: [
            unknown,
            { AddContour: { contour: { points: { x: number }[] } } },
          ]
        }
      }
    }

    expect(
      outcome.apply?.command.Batch[1].AddContour.contour.points.map(
        (point) => point.x,
      ),
    ).toEqual([100, 0, -100])
  })

  it('cancels a path of fewer than two nodes on Escape', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    const outcome = pen.keyDown(
      { key: 'Escape', modifiers: NO_MODIFIERS, repeat: false },
      context,
    )
    expect(outcome.apply).toBeUndefined()
    expect(pen.nodeCount).toBe(0)
  })

  it('finishes an open path of two or more nodes on Escape', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    click(pen, context, { x: 100, y: 0 })
    const outcome = pen.keyDown(
      { key: 'Escape', modifiers: NO_MODIFIERS, repeat: false },
      context,
    ) as {
      apply?: { command: { AddContour: { contour: { closed: boolean } } } }
    }
    expect(outcome.apply?.command.AddContour.contour.closed).toBe(false)
  })

  it('commits what it has when the tool is put away', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    click(pen, context, { x: 100, y: 0 })
    const outcome = pen.finish(context) as {
      apply?: { command: { AddContour: unknown } }
    }
    expect(outcome.apply?.command).toHaveProperty('AddContour')
  })

  it('draws quadratic segments on a quadratic layer', () => {
    const context = makeToolContext({ curves: 'quadratic' })
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    pen.pointerDown(toolPointer(context, { x: 200, y: 0 }), context)
    pen.pointerMove(toolPointer(context, { x: 240, y: 40 }), context)
    pen.pointerUp(toolPointer(context, { x: 240, y: 40 }), context)
    click(pen, context, { x: 300, y: 0 })
    const outcome = pen.keyDown(
      { key: 'Enter', modifiers: NO_MODIFIERS, repeat: false },
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
    // Three nodes, two segments, one off-curve control each — never the two
    // controls a cubic would use.
    expect(kinds).toEqual(['Move', 'OffCurve', 'QCurve', 'OffCurve', 'QCurve'])
  })

  it('shows a rubber band to the pointer while drawing', () => {
    const context = makeToolContext()
    const pen = new PenTool()
    click(pen, context, { x: 0, y: 0 })
    click(pen, context, { x: 100, y: 0 })
    pen.pointerMove(toolPointer(context, { x: 100, y: 100 }), context)
    const overlay = pen.overlay(context)
    expect(overlay.contours).toHaveLength(1)
    expect(overlay.contours[0]?.points.map((point) => point.y)).toEqual([
      0, 0, 100,
    ])
  })

  it('names the cursor variant for what the click would do', () => {
    const open = makeContour(
      [makePoint(0, 0, 'Move'), makePoint(100, 0, 'Line')],
      false,
    )
    const context = makeToolContext({ contours: [open] })
    const pen = new PenTool()
    const over = (font: { x: number; y: number }) =>
      pen.cursor(
        { screen: context.viewport.toScreen(font), modifiers: NO_MODIFIERS },
        context,
      )
    expect(over({ x: 100, y: 0 })).toContain('crosshair')
    expect(over({ x: 50, y: 0 })).toContain('copy')
    click(pen, context, { x: 300, y: 300 })
    click(pen, context, { x: 320, y: 300 })
    expect(over({ x: 300, y: 300 })).toContain('crosshair')
  })
})
