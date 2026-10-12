/** Freehand polygon selection, with the same Alt/Shift grammar as marquee. */
import type { Point2 } from '../../canvas/Viewport'
import {
  selectionOf,
  unionSelection,
  type EditTarget,
  type Selection,
} from '../selection'
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

/** Includes points on a polygon edge, useful when a lasso lands on a node. */
export function polygonContains(
  polygon: readonly Point2[],
  point: Point2,
): boolean {
  if (polygon.length < 3) return false
  let inside = false
  for (
    let index = 0, previous = polygon.length - 1;
    index < polygon.length;
    previous = index++
  ) {
    const a = polygon[index]!
    const b = polygon[previous]!
    const cross = (point.x - a.x) * (b.y - a.y) - (point.y - a.y) * (b.x - a.x)
    if (
      Math.abs(cross) < 1e-7 &&
      point.x >= Math.min(a.x, b.x) &&
      point.x <= Math.max(a.x, b.x) &&
      point.y >= Math.min(a.y, b.y) &&
      point.y <= Math.max(a.y, b.y)
    )
      return true
    if (
      a.y > point.y !== b.y > point.y &&
      point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y) + a.x
    )
      inside = !inside
  }
  return inside
}

/** Freehand selection never mutates the document or creates an undo entry. */
export class LassoTool implements EditorTool {
  readonly id = 'lasso' as const
  private path: Point2[] = []
  private additive = false
  private ignoreHandles = false
  private original: Selection = new Map()

  get busy(): boolean {
    return this.path.length > 0
  }
  claimsKey(key: string): boolean {
    return this.busy && key === 'Escape'
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    this.path = [pointer.font]
    this.additive = pointer.modifiers.shift || pointer.modifiers.mod
    this.ignoreHandles = pointer.modifiers.alt
    this.original = context.selection
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const previous = this.path.at(-1)
    if (!previous) return NO_OUTCOME
    if (
      Math.hypot(previous.x - pointer.font.x, previous.y - pointer.font.y) *
        context.viewport.scale >=
      1
    )
      this.path.push(pointer.font)
    return { handled: true }
  }

  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.busy) return NO_OUTCOME
    this.pointerMove(pointer, context)
    const targets: EditTarget[] = []
    for (const point of context.outline.points) {
      if (
        (!point.onCurve && this.ignoreHandles) ||
        !polygonContains(this.path, point)
      )
        continue
      targets.push({ kind: point.onCurve ? 'node' : 'handle', ...point.ref })
    }
    context.anchors.forEach((point, index) => {
      if (polygonContains(this.path, point))
        targets.push({ kind: 'anchor', index })
    })
    context.components.forEach((component, index) => {
      if (!component.bbox) return
      const [x0, y0, x1, y1] = component.bbox
      if (
        [
          { x: x0, y: y0 },
          { x: x1, y: y0 },
          { x: x0, y: y1 },
          { x: x1, y: y1 },
        ].every((point) => polygonContains(this.path, point))
      )
        targets.push({ kind: 'component', index })
    })
    const found = selectionOf(
      targets.filter((target) => context.canSelect?.(target) !== false),
    )
    const selection = this.additive
      ? unionSelection(this.original, found)
      : found
    this.finish()
    return { handled: true, selection }
  }

  keyDown(key: ToolKey): ToolOutcome {
    if (!this.claimsKey(key.key)) return NO_OUTCOME
    this.finish()
    return { handled: true }
  }

  finish(): ToolOutcome {
    this.path = []
    return NO_OUTCOME
  }
  cursor(): string {
    return 'crosshair'
  }
  overlay(): ToolOverlay {
    if (!this.busy) return NO_TOOL_OVERLAY
    return {
      ...NO_TOOL_OVERLAY,
      lines: this.path.map((from, index) => ({
        from,
        to: this.path[(index + 1) % this.path.length]!,
        dashed: true,
      })),
    }
  }
}
