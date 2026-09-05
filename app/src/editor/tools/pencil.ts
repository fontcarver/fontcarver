/**
 * The Pencil tool (design spec §5).
 *
 * The stroke is sampled locally while the pointer moves and committed once, as
 * a polyline plus a simplify pass: `Batch([AddContour, PathOp::Simplify])`,
 * which the kernel coalesces into one undo entry called "Draw". Fitting the
 * curve in the kernel rather than the browser keeps one implementation of
 * `fw-geom`'s simplifier as the only one in the product.
 *
 * Shift holds the stroke straight: while it is down the newest sample replaces
 * the previous one instead of being appended, so the span from the last corner
 * to the pointer stays a line.
 */

import type { Command, Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import type { SimplifyOptions } from '../../api'

import type { LayerTarget } from '../pointCommands'

import { addContour } from './pen'
import type { ToolId } from './registry'
import {
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** Samples closer together than this, in CSS pixels, are dropped. */
export const PENCIL_SAMPLE_SPACING = 2

/** A stroke whose ends come this close, in CSS pixels, is closed. */
export const PENCIL_CLOSE_DISTANCE = 10

/**
 * How far the simplifier may move the fitted curve, in font units.
 *
 * `Prefs` has no simplify tolerance of its own, so this is the ruling's
 * fallback of one unit.
 */
export const PENCIL_SIMPLIFY_ERROR = 1

/** The simplify pass a freehand stroke is fitted with. */
export function pencilSimplifyOptions(error: number): SimplifyOptions {
  return {
    error,
    ignore_slopes: false,
    // Extrema are left alone: the op runs over the whole layer, and adding
    // nodes to contours the user did not draw would be a surprise.
    ignore_extrema: true,
    smooth_curves: true,
    force_lines: true,
    nearly_hv_lines: false,
    start_at_extrema: false,
  }
}

/** The polyline contour a run of samples describes. */
export function polylineContour(
  samples: readonly Point2[],
  closed: boolean,
): Contour | null {
  if (samples.length < 2) return null
  return {
    closed,
    points: samples.map((point, index) => ({
      x: point.x,
      y: point.y,
      kind: !closed && index === 0 ? ('Move' as const) : ('Line' as const),
      smooth: false,
      name: null,
      tt_index: 'Unnumbered' as const,
    })),
    name: null,
    id: null,
  }
}

export class PencilTool implements EditorTool {
  readonly id: ToolId = 'pencil'

  private samples: Point2[] = []
  private drawing = false
  /** Whether the newest sample was laid down while Shift was held. */
  private straight = false

  get busy(): boolean {
    return this.drawing
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    this.samples = [context.snap(pointer.font, pointer.modifiers.mod).point]
    this.drawing = true
    this.straight = false
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.drawing) return NO_OUTCOME
    const at = context.snap(pointer.font, pointer.modifiers.mod).point
    const last = this.samples[this.samples.length - 1]
    if (!last) return { handled: true }
    if (pointer.modifiers.shift) {
      // Straight span: keep replacing the free end instead of adding to it.
      if (this.straight) this.samples[this.samples.length - 1] = at
      else this.samples.push(at)
      this.straight = true
      return { handled: true }
    }
    this.straight = false
    const scale = context.viewport.scale || 1
    if (
      Math.hypot(at.x - last.x, at.y - last.y) * scale <
      PENCIL_SAMPLE_SPACING
    )
      return { handled: true }
    this.samples.push(at)
    return { handled: true }
  }

  pointerUp(_pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.drawing) return NO_OUTCOME
    return { ...this.commit(context), handled: true }
  }

  finish(context: ToolContext): ToolOutcome {
    if (!this.drawing) return NO_OUTCOME
    return this.commit(context)
  }

  overlay(context: ToolContext): ToolOverlay {
    const contour = polylineContour(this.samples, false)
    if (!contour) return NO_TOOL_OVERLAY
    void context
    return { ...NO_TOOL_OVERLAY, contours: [contour] }
  }

  cursor(): string {
    // §2.4 gives the shape and freehand tools the plain crosshair; the custom
    // cursors are for the pen, the knife, the measure and the transforms.
    return 'crosshair'
  }

  private commit(context: ToolContext): ToolOutcome {
    const samples = this.samples
    this.samples = []
    this.drawing = false
    this.straight = false
    const target = context.target
    if (!target || samples.length < 2) return NO_OUTCOME
    const first = samples[0] as Point2
    const last = samples[samples.length - 1] as Point2
    const scale = context.viewport.scale || 1
    const closed =
      samples.length > 2 &&
      Math.hypot(last.x - first.x, last.y - first.y) * scale <
        PENCIL_CLOSE_DISTANCE
    const contour = polylineContour(
      closed ? samples.slice(0, -1) : samples,
      closed,
    )
    if (!contour) return NO_OUTCOME
    return {
      apply: {
        command: drawCommand(target, contour),
        label: context.labels.draw,
      },
    }
  }
}

/** The polyline and the simplify pass that fits it, as one undo entry. */
export function drawCommand(target: LayerTarget, contour: Contour): Command {
  return {
    Batch: [
      addContour(target, contour),
      {
        PathOp: {
          glyph: target.glyph,
          layer: target.layer,
          op: { Simplify: pencilSimplifyOptions(PENCIL_SIMPLIFY_ERROR) },
        },
      },
    ],
  }
}
