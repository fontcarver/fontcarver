/**
 * The Measure tool (design spec §5, §6.2).
 *
 * The line remains local until G drops a guide: the drag draws a line, the HUD reads its
 * length and angle, and every crossing with the outline is marked with the
 * distance to the previous one, which is how a designer checks a stem width or
 * a counter without changing anything.
 */

import type { Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

import { controlModifier } from '../gestures/drag'
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
  type ToolKey,
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
  const sorted = hits.sort((a, b) => a.s - b.s).map((hit) => hit.point)
  return sorted.filter((point, index) => {
    const previous = sorted[index - 1]
    return (
      !previous || Math.hypot(point.x - previous.x, point.y - previous.y) > 1e-7
    )
  })
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
  /** Whether the button is down; the measurement outlives the gesture. */
  private dragging = false

  get busy(): boolean {
    return this.dragging
  }

  claimsKey(key: string): boolean {
    return (
      (key === 'Escape' && this.from !== null) ||
      (key.toLowerCase() === 'g' && this.dragging)
    )
  }

  keyDown(key: ToolKey, context: ToolContext): ToolOutcome {
    if (key.key === 'Escape' && this.from) {
      this.finish()
      return { handled: true }
    }
    if (
      key.key.toLowerCase() !== 'g' ||
      !this.dragging ||
      key.modifiers.mod ||
      key.modifiers.alt ||
      key.repeat
    )
      return NO_OUTCOME
    const { from, to } = this
    const target = context.target
    if (
      !from ||
      !to ||
      !target ||
      Math.hypot(to.x - from.x, to.y - from.y) < 1e-7
    )
      return { handled: true }
    return {
      handled: true,
      apply: {
        label: context.labels.addGuide,
        command: {
          AddGuide: {
            glyph: target.glyph,
            layer: target.layer,
            master: null,
            guide: {
              ...from,
              angle:
                ((Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI +
                  360) %
                180,
              name: null,
              locked: false,
              color: null,
            },
          },
        },
      },
    }
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    this.from = context.snap(
      pointer.font,
      controlModifier(pointer.modifiers),
    ).point
    this.to = this.from
    this.dragging = true
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from || !this.dragging) return NO_OUTCOME
    const at = context.snap(
      pointer.font,
      controlModifier(pointer.modifiers),
    ).point
    this.to = pointer.modifiers.shift ? constrainVector(this.from, at) : at
    return { handled: true }
  }

  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from) return NO_OUTCOME
    // The measurement stays on screen after the button comes up, so the
    // numbers can be read; the next press starts a new one.
    const outcome = this.pointerMove(pointer, context)
    this.dragging = false
    return { ...outcome, handled: true }
  }

  finish(): ToolOutcome {
    this.from = null
    this.to = null
    this.dragging = false
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

/** Delta in the glyph's upright coordinate system (OpenType italic angle). */
export function measureDelta(
  from: Point2,
  to: Point2,
  italicAngle = 0,
): Point2 {
  const y = to.y - from.y
  const angle =
    Number.isFinite(italicAngle) && Math.abs(italicAngle) < 89 ? italicAngle : 0
  return { x: to.x - from.x + y * Math.tan((angle * Math.PI) / 180), y }
}

/** Distance, physical angle, and italic-aware coordinate deltas. */
export function measureHud(
  from: Point2,
  to: Point2,
  context: ToolContext,
): string {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const angle = (Math.atan2(dy, dx) * 180) / Math.PI
  const delta = measureDelta(from, to, context.italicAngle)
  return (
    `${context.labels.distance} ${formatUnits(Math.hypot(dx, dy))}` +
    `  ${context.labels.hud.angle} ${angle.toFixed(1)}°` +
    `  ${context.labels.hud.deltaX} ${formatUnits(delta.x)}  ${context.labels.hud.deltaY} ${formatUnits(delta.y)}`
  )
}
