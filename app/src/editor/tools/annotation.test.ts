import { describe, expect, it } from 'vitest'
import type { AnnotationKind, LayerAnnotation } from '../../api'
import { makeToolContext, toolPointer } from '../../test/toolContext'
import { NO_MODIFIERS } from '../gestures/drag'
import { AnnotationTool, type AnnotationToolSettings } from './annotation'

function settings(kind: AnnotationKind = 'Text'): AnnotationToolSettings {
  return {
    kind,
    text: 'Stem review',
    annotations: [],
    label: 'Edit annotations',
    locked: false,
  }
}

const existing: LayerAnnotation = {
  id: 'first',
  kind: 'Text',
  text: 'Old',
  x: 0,
  y: 0,
  end_x: 0,
  end_y: 0,
}

describe('annotation canvas tool', () => {
  it.each(['Text', 'Arrow', 'Circle', 'Rectangle', 'Plus', 'Minus'] as const)(
    'stores %s marks as one layer edit',
    (kind) => {
      let current = settings(kind)
      const tool = new AnnotationTool(() => current)
      const context = makeToolContext()
      tool.pointerDown(toolPointer(context, { x: 10, y: 20 }), context)
      tool.pointerMove(toolPointer(context, { x: 40, y: 60 }), context)
      expect(tool.overlay().annotations?.[0]).toMatchObject({
        kind,
        x: 10,
        y: 20,
        end_x: 40,
        end_y: 60,
      })
      current = { ...current, annotations: [existing] }
      const outcome = tool.pointerUp(
        toolPointer(context, { x: 50, y: 70 }),
        context,
      )
      expect(outcome.apply?.command).toEqual({
        EditorMetadata: {
          Annotations: {
            glyph: context.target!.glyph,
            layer: context.target!.layer,
            annotations: [
              existing,
              expect.objectContaining({
                kind,
                x: 10,
                y: 20,
                end_x: 50,
                end_y: 70,
                text: 'Stem review',
              }),
            ],
          },
        },
      })
      expect(tool.busy).toBe(false)
    },
  )

  it('cancels a gesture without editing and refuses locked glyphs', () => {
    const context = makeToolContext()
    const tool = new AnnotationTool(() => settings())
    tool.pointerDown(toolPointer(context, { x: 10, y: 20 }), context)
    expect(
      tool.keyDown({ key: 'Escape', modifiers: NO_MODIFIERS, repeat: false }),
    ).toEqual({ handled: true })
    expect(
      tool.pointerUp(toolPointer(context, { x: 10, y: 20 }), context),
    ).toEqual({})
    const locked = new AnnotationTool(() => ({ ...settings(), locked: true }))
    expect(
      locked.pointerDown(toolPointer(context, { x: 10, y: 20 }), context),
    ).toEqual({})
    expect(locked.busy).toBe(false)
  })
  it('does not apply a gesture to a different layer after source handover', () => {
    const context = makeToolContext()
    const tool = new AnnotationTool(() => settings())
    tool.pointerDown(toolPointer(context, { x: 10, y: 20 }), context)
    expect(
      tool.pointerUp(toolPointer(context, { x: 30, y: 40 }), {
        ...context,
        target: { ...context.target!, layer: 'different-layer' },
      }).apply,
    ).toBeUndefined()
    expect(tool.busy).toBe(false)
  })
})
