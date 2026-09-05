/**
 * The Pencil tool (design spec §5).
 *
 * The stroke is sampled locally while the pointer moves, fitted to cubics by
 * `curveFit.ts` and committed as one `AddContour` called "Draw". The fit is
 * local because the kernel's only whole-path simplifier rewrites *every*
 * contour of the layer, so a stroke drawn into a glyph that already has
 * outlines would silently refit those too.
 *
 * Shift holds the stroke straight: while it is down the newest sample replaces
 * the previous one instead of being appended, so the span from the last corner
 * to the pointer stays a line.
 */

import type { Command, Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

import { DEFAULT_FIT_ERROR, fitContour } from '../curveFit'

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
 * How far the fitted curve may stray from a sample, in font units.
 *
 * `Prefs` has no fitting tolerance of its own, so this is the ruling's
 * fallback of one unit.
 */
export const PENCIL_FIT_ERROR = DEFAULT_FIT_ERROR

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

  overlay(): ToolOverlay {
    // The raw samples, not the fit: the stroke has to follow the pointer
    // exactly, and the fit is what the release commits.
    const contour = polylineContour(this.samples, false)
    if (!contour) return NO_TOOL_OVERLAY
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
    // A closed stroke is fitted through its own start, so the last segment
    // really ends where the contour begins rather than near it.
    const contour = fitContour(
      closed ? [...samples, first] : samples,
      closed,
      context.curves,
      PENCIL_FIT_ERROR,
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

/** The fitted stroke, as one undo entry that touches nothing else. */
export function drawCommand(target: LayerTarget, contour: Contour): Command {
  return addContour(target, contour)
}
