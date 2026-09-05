import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type ApiMethods } from '../../api'
import { makeContour, makePoint } from '../../test/fontFixtures'
import {
  createEditingTransport,
  type MockTransport,
} from '../../test/mockTransport'
import { makeToolContext, toolPointer } from '../../test/toolContext'
import { NO_MODIFIERS } from '../gestures/drag'
import { selectionOf } from '../selection'

import { RotateTool } from './rotate'
import { ScaleTool } from './scale'
import { defaultOrigin } from './transformTool'
import type { ToolContext } from './toolTypes'

const shift = { ...NO_MODIFIERS, shift: true }

/** A 200 × 200 square whose centre is (100, 100). */
const square = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(200, 200, 'Line'),
  makePoint(0, 200, 'Line'),
])

function install(): MockTransport {
  const transport = createEditingTransport()
  setTransport(transport)
  return transport
}

/** Waits for the promise chain a gesture started. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0))
  await new Promise((resolve) => setTimeout(resolve, 0))
}

afterEach(async () => {
  await settle()
  setTransport(undefined)
})

/** The affine of every `TransformSelection` the gesture sent. */
function affines(transport: MockTransport): number[][] {
  return transport.calls
    .filter((entry) => entry.method === 'doc.edit_apply')
    .map((entry) => {
      const params = entry.params as ApiMethods['doc.edit_apply']['params']
      const command = params.command as {
        TransformSelection: { affine: number[]; points: unknown[] }
      }
      return command.TransformSelection.affine
    })
}

function selectionContext(): ToolContext {
  const context = makeToolContext({ contours: [square] })
  return {
    ...context,
    selection: selectionOf([
      { kind: 'node', contour: 0, point: 1 },
      { kind: 'node', contour: 0, point: 2 },
    ]),
  }
}

describe('RotateTool', () => {
  it('turns the selection about the origin and commits one entry', async () => {
    const transport = install()
    const context = selectionContext()
    const tool = new RotateTool()
    // A click first names the origin, then a drag turns about it.
    const at = (x: number, y: number) => toolPointer(context, { x, y })
    tool.pointerDown(at(0, 0), context)
    tool.pointerUp()
    tool.pointerDown(at(100, 0), context)
    tool.pointerMove(at(0, 100), context)
    const outcome = tool.pointerUp()
    await outcome.closing
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    // A quarter turn about (0, 0): [cos, sin, −sin, cos, 0, 0].
    const affine = affines(transport)[0] ?? []
    expect(affine[0]).toBeCloseTo(0, 9)
    expect(affine[1]).toBeCloseTo(1, 9)
    expect(affine[2]).toBeCloseTo(-1, 9)
    expect(affine[3]).toBeCloseTo(0, 9)
  })

  it('previews the turn before the kernel answers', () => {
    install()
    const context = selectionContext()
    const tool = new RotateTool()
    const at = (x: number, y: number) => toolPointer(context, { x, y })
    tool.pointerDown(at(0, 0), context)
    tool.pointerUp()
    tool.pointerDown(at(100, 0), context)
    const outcome = tool.pointerMove(at(0, 100), context)
    const points = outcome.preview?.contours[0]?.points ?? []
    // Only the selected nodes move; (200, 0) turns to (0, 200).
    expect(points[0]).toMatchObject({ x: 0, y: 0 })
    expect(points[1]?.x).toBeCloseTo(0, 9)
    expect(points[1]?.y).toBeCloseTo(200, 9)
    void tool.pointerUp()
  })

  it('snaps the turn to 15° steps with Shift', () => {
    install()
    const context = selectionContext()
    const tool = new RotateTool()
    const at = (x: number, y: number, modifiers = NO_MODIFIERS) =>
      toolPointer(context, { x, y }, modifiers)
    tool.pointerDown(at(0, 0), context)
    tool.pointerUp()
    tool.pointerDown(at(100, 0), context)
    tool.pointerMove(at(100, 18, shift), context)
    // 10.2° rounds to 15°, not to the raw angle.
    expect(tool.overlay(context).hud).toBe('∠ 15.0°')
    void tool.pointerUp()
  })

  it('defaults its origin to the centre of the selection', () => {
    const context = selectionContext()
    expect(defaultOrigin(context)).toEqual({ x: 200, y: 100 })
  })

  it('falls back to the centre of the layer with nothing selected', () => {
    const context = makeToolContext({ contours: [square] })
    expect(defaultOrigin(context)).toEqual({ x: 100, y: 100 })
  })

  it('transforms the whole layer when nothing is selected', async () => {
    const transport = install()
    const context = makeToolContext({ contours: [square] })
    const tool = new RotateTool()
    const at = (x: number, y: number) => toolPointer(context, { x, y })
    tool.pointerDown(at(200, 100), context)
    tool.pointerMove(at(100, 200), context)
    const outcome = tool.pointerUp()
    await outcome.closing
    await settle()

    const apply = transport.calls.find(
      (entry) => entry.method === 'doc.edit_apply',
    )
    const params = apply?.params as ApiMethods['doc.edit_apply']['params']
    const command = params.command as {
      TransformSelection: {
        points: unknown[]
        anchors: unknown[]
        components: unknown[]
      }
    }
    // Naming nothing is how the kernel is told "the whole layer".
    expect(command.TransformSelection.points).toEqual([])
    expect(command.TransformSelection.anchors).toEqual([])
    expect(command.TransformSelection.components).toEqual([])
  })
})

describe('ScaleTool', () => {
  it('scales each axis by the ratio the drag changed it by', async () => {
    const transport = install()
    const context = selectionContext()
    const tool = new ScaleTool()
    const at = (x: number, y: number) => toolPointer(context, { x, y })
    tool.pointerDown(at(0, 0), context)
    tool.pointerUp()
    tool.pointerDown(at(100, 100), context)
    tool.pointerMove(at(200, 50), context)
    const outcome = tool.pointerUp()
    await outcome.closing
    await settle()

    const affine = affines(transport)[0] ?? []
    expect(affine[0]).toBeCloseTo(2, 9)
    expect(affine[3]).toBeCloseTo(0.5, 9)
  })

  it('keeps the two axes equal with Shift', () => {
    install()
    const context = selectionContext()
    const tool = new ScaleTool()
    const at = (x: number, y: number, modifiers = NO_MODIFIERS) =>
      toolPointer(context, { x, y }, modifiers)
    tool.pointerDown(at(0, 0), context)
    tool.pointerUp()
    tool.pointerDown(at(100, 0), context)
    tool.pointerMove(at(200, 0, shift), context)
    expect(tool.overlay(context).hud).toBe('⤢ 200.0% × 200.0%')
    void tool.pointerUp()
  })

  it('sends only the step since the previous apply', async () => {
    const transport = install()
    const context = selectionContext()
    const tool = new ScaleTool()
    const at = (x: number, y: number) => toolPointer(context, { x, y })
    tool.pointerDown(at(0, 0), context)
    tool.pointerUp()
    tool.pointerDown(at(100, 100), context)
    tool.pointerMove(at(200, 200), context)
    await settle()
    tool.pointerMove(at(400, 400), context)
    const outcome = tool.pointerUp()
    await outcome.closing
    await settle()

    const sent = affines(transport)
    expect(sent).toHaveLength(2)
    // ×2 then ×2 again, not ×2 then ×4: `TransformSelection` transforms the
    // current state, so each apply carries the step.
    expect(sent[0]?.[0]).toBeCloseTo(2, 9)
    expect(sent[1]?.[0]).toBeCloseTo(2, 9)
  })

  it('abandons the gesture on Escape', async () => {
    const transport = install()
    const context = selectionContext()
    const tool = new ScaleTool()
    const at = (x: number, y: number) => toolPointer(context, { x, y })
    tool.pointerDown(at(100, 100), context)
    tool.pointerMove(at(200, 200), context)
    const outcome = tool.keyDown({
      key: 'Escape',
      modifiers: NO_MODIFIERS,
      repeat: false,
    })
    expect(outcome.preview).toBeNull()
    await outcome.closing
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
    ])
  })
})
