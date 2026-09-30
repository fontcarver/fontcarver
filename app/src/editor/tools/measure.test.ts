import { describe, expect, it } from 'vitest'

import { makeContour, makePoint as point } from '../../test/fontFixtures'
import { makeToolContext, toolPointer } from '../../test/toolContext'
import { NO_MODIFIERS } from '../gestures/drag'

import { MeasureTool, measureCrossings, measureDelta } from './measure'

describe('measurement geometry', () => {
  it('deduplicates shared contour vertices so spans never report a zero length', () => {
    const triangle = makeContour([point(0, 0), point(100, 100), point(200, 0)])
    expect(
      measureCrossings(
        [triangle, triangle],
        { x: -50, y: 0 },
        { x: 250, y: 0 },
      ),
    ).toEqual([
      { x: 0, y: 0 },
      { x: 200, y: 0 },
    ])
  })

  it('reports horizontal displacement in the upright coordinate system of italic glyphs', () => {
    expect(measureDelta({ x: 20, y: 10 }, { x: 120, y: 210 }, 0)).toEqual({
      x: 100,
      y: 200,
    })
    const tilt = Math.tan((12 * Math.PI) / 180)
    const delta = measureDelta({ x: 0, y: 0 }, { x: 200 * tilt, y: 200 }, -12)
    expect(delta.x).toBeCloseTo(0, 9)
    expect(delta.y).toBe(200)
  })
})

describe('measurement guide creation', () => {
  it('drops a guide along the live measurement as one command and keeps measuring', () => {
    const context = makeToolContext()
    const tool = new MeasureTool()
    tool.pointerDown(toolPointer(context, { x: 20, y: 30 }), context)
    tool.pointerMove(toolPointer(context, { x: 120, y: 130 }), context)
    const result = tool.keyDown(
      { key: 'g', modifiers: NO_MODIFIERS, repeat: false },
      context,
    )
    expect(result.apply).toEqual({
      label: 'Add guide',
      command: {
        AddGuide: {
          glyph: 'a',
          layer: 'layer-1',
          master: null,
          guide: {
            x: 20,
            y: 30,
            angle: 45,
            name: null,
            locked: false,
            color: null,
            anchor: 'Origin',
          },
        },
      },
    })
    expect(tool.busy).toBe(true)
    expect(tool.overlay(context).hud).toContain('Δx 100  Δy 100')
    expect(
      tool.keyDown({ key: 'g', modifiers: NO_MODIFIERS, repeat: true }, context)
        .apply,
    ).toBeUndefined()
  })

  it('does not create guides for an empty measurement, command shortcut or idle tool', () => {
    const context = makeToolContext()
    const tool = new MeasureTool()
    expect(tool.claimsKey('g')).toBe(false)
    tool.pointerDown(toolPointer(context, { x: 20, y: 30 }), context)
    expect(
      tool.keyDown(
        { key: 'g', modifiers: NO_MODIFIERS, repeat: false },
        context,
      ).apply,
    ).toBeUndefined()
    tool.pointerMove(toolPointer(context, { x: 120, y: 30 }), context)
    expect(
      tool.keyDown(
        { key: 'g', modifiers: { ...NO_MODIFIERS, mod: true }, repeat: false },
        context,
      ).apply,
    ).toBeUndefined()
    tool.pointerUp(toolPointer(context, { x: 120, y: 30 }), context)
    expect(tool.claimsKey('g')).toBe(false)
  })

  it('Escape removes the measurement immediately', () => {
    const context = makeToolContext()
    const tool = new MeasureTool()
    tool.pointerDown(toolPointer(context, { x: 20, y: 30 }), context)
    tool.keyDown(
      { key: 'Escape', modifiers: NO_MODIFIERS, repeat: false },
      context,
    )
    expect(tool.busy).toBe(false)
    expect(tool.overlay(context).hud).toBeNull()
  })
})
