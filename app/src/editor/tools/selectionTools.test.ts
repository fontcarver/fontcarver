import { describe, expect, it } from 'vitest'

import { makeContour, makePoint as point } from '../../test/fontFixtures'
import { makeToolContext, toolPointer } from '../../test/toolContext'
import { NO_MODIFIERS } from '../gestures/drag'
import { selectionOf } from '../selection'

import { EraserTool } from './eraser'
import { LassoTool, polygonContains } from './lasso'

function lasso(points: { x: number; y: number }[], modifiers = NO_MODIFIERS) {
  const context = makeToolContext({
    contours: [
      makeContour(
        [point(20, 20), point(60, 20, 'OffCurve'), point(80, 80)],
        false,
      ),
    ],
  })
  const tool = new LassoTool()
  tool.pointerDown(toolPointer(context, points[0]!, modifiers), context)
  points
    .slice(1)
    .forEach((p) => tool.pointerMove(toolPointer(context, p), context))
  return {
    tool,
    context,
    result: tool.pointerUp(toolPointer(context, points.at(-1)!), context),
  }
}

describe('lasso selection', () => {
  it('uses the polygon, including its boundary, rather than the bounding rectangle', () => {
    const path = [
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 0, y: 100 },
    ]
    expect(polygonContains(path, { x: 50, y: 50 })).toBe(true)
    const { result } = lasso(path)
    expect([...result.selection!.keys()]).toEqual(['node:0:0', 'handle:0:1'])
  })

  it('captures Alt on press to leave handles out', () => {
    const { result } = lasso(
      [
        { x: 0, y: 0 },
        { x: 100, y: 0 },
        { x: 0, y: 100 },
      ],
      { ...NO_MODIFIERS, alt: true },
    )
    expect([...result.selection!.keys()]).toEqual(['node:0:0'])
  })

  it('preserves the existing selection with Shift and cancels with Escape', () => {
    const context = makeToolContext()
    context.selection = selectionOf([{ kind: 'anchor', index: 0 }])
    const tool = new LassoTool()
    tool.pointerDown(
      toolPointer(context, { x: 0, y: 0 }, { ...NO_MODIFIERS, shift: true }),
      context,
    )
    expect(
      tool.pointerUp(toolPointer(context, { x: 0, y: 0 }), context).selection,
    ).toEqual(context.selection)
    tool.pointerDown(toolPointer(context, { x: 0, y: 0 }), context)
    expect(
      tool.keyDown({ key: 'Escape', modifiers: NO_MODIFIERS, repeat: false })
        .handled,
    ).toBe(true)
    expect(tool.busy).toBe(false)
  })
})

describe('eraser stroke', () => {
  it('collects nodes crossed by the complete stroke and commits once on release', () => {
    const context = makeToolContext({
      contours: [
        makeContour(
          [point(0, 0), point(50, 0), point(100, 0), point(200, 0)],
          false,
        ),
      ],
    })
    const tool = new EraserTool()
    expect(
      tool.pointerDown(toolPointer(context, { x: -20, y: 0 }), context).apply,
    ).toBeUndefined()
    expect(
      tool.pointerMove(toolPointer(context, { x: 120, y: 0 }), context).apply,
    ).toBeUndefined()
    const result = tool.pointerUp(
      toolPointer(context, { x: 120, y: 0 }),
      context,
    )
    expect(result.apply).toEqual({
      label: 'Erase',
      command: {
        DeletePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: [
            { contour: 0, point: 0 },
            { contour: 0, point: 1 },
            { contour: 0, point: 2 },
          ],
          mode: 'KeepShape',
        },
      },
    })
    expect(tool.busy).toBe(false)
  })

  it('uses Shift for the gap policy and Escape discards an uncommitted stroke', () => {
    const context = makeToolContext({
      contours: [makeContour([point(0, 0), point(100, 0)], false)],
    })
    const tool = new EraserTool()
    tool.pointerDown(
      toolPointer(context, { x: 0, y: 0 }, { ...NO_MODIFIERS, shift: true }),
      context,
    )
    const result = tool.pointerUp(toolPointer(context, { x: 0, y: 0 }), context)
    expect(result.apply?.command).toMatchObject({
      DeletePoints: { mode: 'BreakContour' },
    })
    tool.pointerDown(toolPointer(context, { x: 100, y: 0 }), context)
    tool.keyDown({ key: 'Escape', modifiers: NO_MODIFIERS, repeat: false })
    expect(
      tool.pointerUp(toolPointer(context, { x: 100, y: 0 }), context).apply,
    ).toBeUndefined()
  })
})
