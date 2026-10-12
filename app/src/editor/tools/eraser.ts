/** A stroke collects nodes; release deletes the collection in one undo step. */
import type { DeleteMode, PointRef } from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import { deleteCommand } from '../pointCommands'
import { EMPTY_SELECTION } from '../selection'
import {
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolKey,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** Circular brush radius in CSS pixels, independent of font zoom. */
const ERASE_RADIUS = 6

function distanceToStroke(point: Point2, from: Point2, to: Point2): number {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const squared = dx * dx + dy * dy
  const t =
    squared === 0
      ? 0
      : Math.max(
          0,
          Math.min(
            1,
            ((point.x - from.x) * dx + (point.y - from.y) * dy) / squared,
          ),
        )
  return Math.hypot(point.x - from.x - t * dx, point.y - from.y - t * dy)
}

/** Erases on-curve nodes; Shift leaves a gap instead of fitting the path. */
export class EraserTool implements EditorTool {
  readonly id = 'eraser' as const
  private previous: Point2 | null = null
  private points = new Map<string, { ref: PointRef; at: Point2 }>()
  private mode: DeleteMode = 'KeepShape'

  get busy(): boolean {
    return this.previous !== null
  }
  claimsKey(key: string): boolean {
    return this.busy && key === 'Escape'
  }
  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0 || !context.target) return NO_OUTCOME
    this.points.clear()
    this.previous = pointer.font
    this.mode = pointer.modifiers.shift ? 'BreakContour' : 'KeepShape'
    return this.pointerMove(pointer, context)
  }
  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.previous) return NO_OUTCOME
    const reach = ERASE_RADIUS / Math.max(context.viewport.scale, 0.00001)
    for (const point of context.outline.points) {
      if (
        !point.onCurve ||
        context.canSelect?.({ kind: 'node', ...point.ref }) === false
      )
        continue
      if (distanceToStroke(point, this.previous, pointer.font) <= reach)
        this.points.set(`${point.ref.contour}:${point.ref.point}`, {
          ref: point.ref,
          at: point,
        })
    }
    this.previous = pointer.font
    return { handled: true }
  }
  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.busy) return NO_OUTCOME
    this.pointerMove(pointer, context)
    const refs = [...this.points.values()].map(({ ref }) => ref)
    const apply =
      context.target && refs.length
        ? {
            command: deleteCommand(context.target, refs, this.mode),
            label: context.labels.erase,
          }
        : undefined
    this.finish()
    return {
      handled: true,
      apply,
      ...(apply ? { selection: EMPTY_SELECTION } : {}),
    }
  }
  keyDown(key: ToolKey): ToolOutcome {
    if (!this.claimsKey(key.key)) return NO_OUTCOME
    this.finish()
    return { handled: true }
  }
  finish(): ToolOutcome {
    this.previous = null
    this.points.clear()
    return NO_OUTCOME
  }
  cursor(): string {
    return 'crosshair'
  }
  overlay(): ToolOverlay {
    if (!this.busy) return NO_TOOL_OVERLAY
    return {
      ...NO_TOOL_OVERLAY,
      marks: [...this.points.values()].map(({ at }) => at),
    }
  }
}
