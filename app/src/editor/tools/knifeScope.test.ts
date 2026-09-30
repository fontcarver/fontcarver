import { describe, expect, it } from 'vitest'
import { makeContour, makePoint } from '../../test/fontFixtures'
import { makeToolContext, toolPointer } from '../../test/toolContext'
import { KnifeTool } from './knife'
import type { EditTarget } from '../selection'

const contours = [0, 300].map((x) =>
  makeContour([
    makePoint(x, 0),
    makePoint(x + 200, 0),
    makePoint(x + 200, 200),
    makePoint(x, 200),
  ]),
)

describe('knife editing scope', () => {
  it('keeps contour indexes stable while skipping locked or unfocused paths', () => {
    const context = {
      ...makeToolContext({ contours }),
      canSelect: (target: EditTarget) =>
        'contour' in target && target.contour === 1,
    }
    const knife = new KnifeTool()
    knife.pointerDown(toolPointer(context, { x: -50, y: 100 }), context)
    knife.pointerMove(toolPointer(context, { x: 550, y: 100 }), context)
    expect(knife.overlay(context).marks.every((point) => point.x >= 300)).toBe(
      true,
    )
    const outcome = knife.pointerUp(
      toolPointer(context, { x: 550, y: 100 }),
      context,
    )
    expect(outcome.apply?.command).toMatchObject({
      Batch: [
        { DeleteContours: { contours: [1] } },
        { AddContour: {} },
        { AddContour: {} },
      ],
    })
  })

  it('does not mutate a locked glyph when no contour is eligible', () => {
    const context = { ...makeToolContext({ contours }), canSelect: () => false }
    const knife = new KnifeTool()
    knife.pointerDown(toolPointer(context, { x: -50, y: 100 }), context)
    expect(
      knife.pointerUp(toolPointer(context, { x: 550, y: 100 }), context).apply,
    ).toBeUndefined()
  })
})
