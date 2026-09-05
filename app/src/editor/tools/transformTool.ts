/**
 * What the Rotate and Scale tools share (design spec §5).
 *
 * A click sets the origin the transform turns about — the selection's centre
 * until the user picks another — and a drag from anywhere transforms the
 * selection about it, previewed locally and streamed to the kernel as
 * `TransformSelection` steps, so the whole gesture is one undo entry. With
 * nothing selected the command names no points at all, which is how the kernel
 * is told to transform the entire layer.
 */

import type { Point2 } from '../../canvas/Viewport'
import type { FontBox } from '../../canvas/Viewport'

import { toolCursor, type CursorId } from '../cursors'
import type { Modifiers } from '../gestures/drag'
import {
  TransformGesture,
  type TransformAmount,
  type TransformContext,
} from '../gestures/transform'
import { boundsOf, movedPointRefs } from '../outlineGeometry'
import {
  selectedIndexes,
  selectedPointRefs,
  type Selection,
} from '../selection'

import type { ToolId } from './registry'
import {
  movedEnough,
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolKey,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** Rotation is constrained to this step while Shift is held. */
export const ROTATE_STEP_DEGREES = 15

/** A press that has not yet decided whether it is a click or a drag. */
interface TransformPress {
  at: Point2
  dragging: boolean
}

export abstract class TransformTool implements EditorTool {
  abstract readonly id: ToolId
  protected abstract readonly kind: TransformAmount['kind']
  protected abstract readonly cursorId: CursorId

  private origin: Point2 | null = null
  private press: TransformPress | null = null
  private gesture: TransformGesture | null = null
  private amount: TransformAmount | null = null

  get busy(): boolean {
    return this.gesture !== null
  }

  /** The origin the next drag turns about; read by the overlay and by tests. */
  originIn(context: ToolContext): Point2 {
    return this.origin ?? defaultOrigin(context)
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    // The press is snapped once, here: a click names the origin and a drag
    // measures from it, and both should start from the same point.
    this.press = {
      at: context.snap(pointer.font, pointer.modifiers.mod).point,
      dragging: false,
    }
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const press = this.press
    if (!press) return NO_OUTCOME
    if (
      !press.dragging &&
      !movedEnough(press.at, pointer.font, context.viewport.scale)
    )
      return { handled: true }
    press.dragging = true
    const gesture = this.gesture ?? this.begin(context)
    if (!gesture) return { handled: true }
    const amount = this.amountFor(
      this.originIn(context),
      press.at,
      pointer.font,
      pointer.modifiers,
    )
    this.amount = amount
    gesture.update(amount)
    return { preview: gesture.preview, editing: true, handled: true }
  }

  pointerUp(): ToolOutcome {
    const press = this.press
    this.press = null
    if (!press) return NO_OUTCOME
    if (!press.dragging) {
      // A click, not a drag: it names the point the next drag turns about.
      this.origin = press.at
      return { handled: true }
    }
    return { ...this.close(), handled: true }
  }

  keyDown(key: ToolKey): ToolOutcome {
    if (key.key !== 'Escape') return NO_OUTCOME
    const gesture = this.gesture
    if (gesture) {
      this.gesture = null
      this.amount = null
      this.press = null
      return {
        preview: null,
        editing: false,
        closing: gesture.cancel(),
        handled: true,
      }
    }
    if (this.origin === null) return NO_OUTCOME
    this.origin = null
    return { handled: true }
  }

  claimsKey(key: string): boolean {
    return key === 'Escape' && (this.gesture !== null || this.origin !== null)
  }

  finish(): ToolOutcome {
    this.press = null
    this.origin = null
    if (!this.gesture) return NO_OUTCOME
    return this.close()
  }

  overlay(context: ToolContext): ToolOverlay {
    const origin = this.originIn(context)
    return {
      ...NO_TOOL_OVERLAY,
      origin,
      hud: this.amount ? this.hudText(this.amount, context) : null,
    }
  }

  cursor(): string {
    return toolCursor(this.cursorId)
  }

  /** The amount a drag from `from` to `to` about `origin` describes. */
  protected abstract amountFor(
    origin: Point2,
    from: Point2,
    to: Point2,
    modifiers: Modifiers,
  ): TransformAmount

  /** The HUD line for an amount. */
  protected abstract hudText(
    amount: TransformAmount,
    context: ToolContext,
  ): string

  private begin(context: ToolContext): TransformGesture | null {
    const target = context.target
    if (!target) return null
    const refs = selectedPointRefs(context.selection)
    const anchors = selectedIndexes(context.selection, 'anchor')
    const components = selectedIndexes(context.selection, 'component')
    const transformContext: TransformContext = {
      doc: target.doc,
      glyph: target.glyph,
      layer: target.layer,
      contours: context.contours,
      anchors: context.anchors,
      components: context.components,
      // The kernel transforms exactly the points it is given, so a node's
      // handles have to be named explicitly or the curve would tear.
      points:
        refs.length > 0
          ? movedPointRefs(context.contours, refs, 'WithHandles')
          : [],
      anchorIndexes: anchors,
      componentIndexes: components,
      origin: this.originIn(context),
      scheduler: context.scheduler,
      label:
        this.kind === 'rotate' ? context.labels.rotate : context.labels.scale,
      onError: context.onError,
      after: context.after,
    }
    this.gesture = new TransformGesture(transformContext, this.kind)
    return this.gesture
  }

  private close(): ToolOutcome {
    const gesture = this.gesture
    this.gesture = null
    this.amount = null
    if (!gesture) return NO_OUTCOME
    return { preview: null, editing: false, closing: gesture.finish() }
  }
}

/** The centre of what a transform would move: the selection, else the layer. */
export function defaultOrigin(context: ToolContext): Point2 {
  const box = selectionBox(context) ?? layerBox(context)
  if (!box) return { x: 0, y: 0 }
  return { x: (box.x0 + box.x1) / 2, y: (box.y0 + box.y1) / 2 }
}

function selectionBox(context: ToolContext): FontBox | null {
  const points: Point2[] = []
  for (const ref of selectedPointRefs(context.selection)) {
    const point = context.contours[ref.contour]?.points[ref.point]
    if (point) points.push({ x: point.x, y: point.y })
  }
  for (const index of selectedIndexes(context.selection, 'anchor')) {
    const anchor = context.anchors[index]
    if (anchor) points.push({ x: anchor.x, y: anchor.y })
  }
  for (const index of selectedIndexes(context.selection, 'component')) {
    const box = context.components[index]?.bbox
    if (box) points.push({ x: box[0], y: box[1] }, { x: box[2], y: box[3] })
  }
  return boundsOf(points)
}

function layerBox(context: ToolContext): FontBox | null {
  const points: Point2[] = []
  for (const contour of context.contours)
    for (const point of contour.points) points.push({ x: point.x, y: point.y })
  for (const view of context.components)
    if (view.bbox)
      points.push(
        { x: view.bbox[0], y: view.bbox[1] },
        { x: view.bbox[2], y: view.bbox[3] },
      )
  return boundsOf(points)
}

/** Whether a selection names anything a transform would move. */
export function transformsWholeLayer(selection: Selection): boolean {
  return (
    selectedPointRefs(selection).length === 0 &&
    selectedIndexes(selection, 'anchor').length === 0 &&
    selectedIndexes(selection, 'component').length === 0
  )
}
