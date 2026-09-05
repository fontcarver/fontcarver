/**
 * The Measure tool (design spec §5, §6.2).
 *
 * Purely local and purely read-only: the drag draws a line, the HUD reads its
 * length and angle, and every crossing with the outline is marked with the
 * distance to the previous one, which is how a designer checks a stem width or
 * a counter without changing anything.
 */

import type { Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

import { toolCursor } from '../cursors'
import { formatUnits } from '../fieldValue'
import { contourToPath, lineHits } from '../pathGeometry'

import type { ToolId } from './registry'
import {
  constrainVector,
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** One crossing of the measured line, with its distance along it. */
export interface MeasureSpan {
  at: Point2
  text: string
}

/** The crossings of a measured line, in the order the line meets them. */
export function measureCrossings(
  contours: readonly Contour[],
  from: Point2,
  to: Point2,
): Point2[] {
  const hits = contours.flatMap((contour) =>
    lineHits(contourToPath(contour), from, to),
  )
  return hits.sort((a, b) => a.s - b.s).map((hit) => hit.point)
}

/** The per-span labels between consecutive crossings. */
export function measureSpans(points: readonly Point2[]): MeasureSpan[] {
  const spans: MeasureSpan[] = []
  for (let index = 0; index + 1 < points.length; index += 1) {
    const a = points[index] as Point2
    const b = points[index + 1] as Point2
    spans.push({
      at: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
      text: formatUnits(Math.hypot(b.x - a.x, b.y - a.y)),
    })
  }
  return spans
}

export class MeasureTool implements EditorTool {
  readonly id: ToolId = 'measure'

  private from: Point2 | null = null
  private to: Point2 | null = null

  get busy(): boolean {
    return this.from !== null && this.to !== null
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    this.from = context.snap(pointer.font, pointer.modifiers.mod).point
    this.to = this.from
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from) return NO_OUTCOME
    const at = context.snap(pointer.font, pointer.modifiers.mod).point
    this.to = pointer.modifiers.shift ? constrainVector(this.from, at) : at
    return { handled: true }
  }

  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from) return NO_OUTCOME
    // The measurement stays on screen after the button comes up, so the
    // numbers can be read; the next press starts a new one.
    return this.pointerMove(pointer, context)
  }

  finish(): ToolOutcome {
    this.from = null
    this.to = null
    return NO_OUTCOME
  }

  overlay(context: ToolContext): ToolOverlay {
    const { from, to } = this
    if (!from || !to) return NO_TOOL_OVERLAY
    const crossings = measureCrossings(context.contours, from, to)
    return {
      contours: [],
      lines: [{ from, to }],
      marks: crossings,
      labels: measureSpans(crossings),
      origin: null,
      hud: measureHud(from, to, context),
    }
  }

  cursor(): string {
    return toolCursor('measure')
  }
}

/** The `d 312  ∠ 30.0°` line the HUD shows while measuring. */
export function measureHud(
  from: Point2,
  to: Point2,
  context: ToolContext,
): string {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const angle = (Math.atan2(dy, dx) * 180) / Math.PI
  return (
    `${context.labels.distance} ${formatUnits(Math.hypot(dx, dy))}` +
    `  ${context.labels.hud.angle} ${angle.toFixed(1)}°`
  )
}
