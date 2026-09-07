/**
 * The Rectangle and Ellipse tools (design spec §5, §9).
 *
 * A drag draws the shape live — Shift makes it square or circular, Alt grows
 * it from its centre — and commits one `AddContour`. A click without a drag
 * opens the numeric dialog instead, which is the same construction driven by
 * four fields rather than by the pointer.
 */

import type { FontBox, Point2 } from '../../canvas/Viewport'

import type { Modifiers } from '../gestures/drag'
import { hudText } from '../hud'
import { normaliseBox } from '../pathGeometry'
import { boxIsEmpty, shapeContour, type ShapeKind } from '../shapeContours'

import { addContour } from './pen'
import type { ToolId } from './registry'
import {
  movedEnough,
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** The box a drag from `origin` to `current` describes. */
export function shapeBox(
  origin: Point2,
  current: Point2,
  modifiers: Modifiers,
): FontBox {
  let dx = current.x - origin.x
  let dy = current.y - origin.y
  if (modifiers.shift) {
    // A square or a circle: the larger extent wins, in the drag's direction.
    const size = Math.max(Math.abs(dx), Math.abs(dy))
    dx = (dx < 0 ? -1 : 1) * size
    dy = (dy < 0 ? -1 : 1) * size
  }
  if (modifiers.alt)
    // ⌥ grows the shape about the point the drag started from.
    return {
      x0: origin.x - Math.abs(dx),
      y0: origin.y - Math.abs(dy),
      x1: origin.x + Math.abs(dx),
      y1: origin.y + Math.abs(dy),
    }
  return normaliseBox(origin, { x: origin.x + dx, y: origin.y + dy })
}

export class ShapeTool implements EditorTool {
  readonly id: ToolId
  private readonly kind: ShapeKind
  private origin: Point2 | null = null
  private box: FontBox | null = null
  private dragging = false

  constructor(kind: ShapeKind) {
    this.kind = kind
    this.id = kind
  }

  get busy(): boolean {
    return this.origin !== null
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    this.origin = context.snap(pointer.font, pointer.modifiers.mod).point
    this.box = null
    this.dragging = false
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const origin = this.origin
    if (!origin) return NO_OUTCOME
    if (
      !this.dragging &&
      !movedEnough(origin, pointer.font, context.viewport.scale)
    )
      return { handled: true }
    this.dragging = true
    const at = context.snap(pointer.font, pointer.modifiers.mod).point
    this.box = shapeBox(origin, at, pointer.modifiers)
    return { handled: true }
  }

  pointerUp(_pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const origin = this.origin
    const box = this.box
    const dragging = this.dragging
    this.origin = null
    this.box = null
    this.dragging = false
    if (!origin) return NO_OUTCOME
    if (!dragging || !box)
      return { dialog: { kind: this.kind, at: origin }, handled: true }
    return { ...this.commit(context, box), handled: true }
  }

  finish(): ToolOutcome {
    this.origin = null
    this.box = null
    this.dragging = false
    return NO_OUTCOME
  }

  overlay(context: ToolContext): ToolOverlay {
    const box = this.box
    if (!box) return NO_TOOL_OVERLAY
    return {
      ...NO_TOOL_OVERLAY,
      contours: [shapeContour(this.kind, box, context.curves)],
      hud: hudText(
        {
          kind: 'marquee',
          width: box.x1 - box.x0,
          height: box.y1 - box.y0,
        },
        context.labels.hud,
      ),
    }
  }

  cursor(): string {
    // §2.4 gives the shape and freehand tools the plain crosshair; the custom
    // cursors are for the pen, the knife, the measure and the transforms.
    return 'crosshair'
  }

  private commit(context: ToolContext, box: FontBox): ToolOutcome {
    const target = context.target
    if (!target || boxIsEmpty(box)) return NO_OUTCOME
    return {
      apply: {
        command: addContour(
          target,
          shapeContour(this.kind, box, context.curves),
        ),
        label:
          this.kind === 'rectangle'
            ? context.labels.rectangle
            : context.labels.ellipse,
      },
    }
  }
}
