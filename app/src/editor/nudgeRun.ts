/**
 * The lifetime of an arrow-key nudge run.
 *
 * A run is one coalesced edit — one undo entry — that stays open for 500 ms of
 * silence, and while it is open the canvas draws from a local copy so the very
 * first key press moves the outline. Both halves of that belong together:
 * whoever owns the coalescer also owns the accumulated preview, and closing
 * one without the other is what left the editor stuck in an earlier round.
 */

import type { Anchor, Contour, HandleMode, PointRef } from '../api'
import type { Point2 } from '../canvas/Viewport'

import { NudgeCoalescer } from './gestures/nudge'
import { movedPointRefs, translatePoints } from './outlineGeometry'
import type { LayerTarget } from './pointCommands'

/** One arrow press. */
export interface NudgePress {
  target: LayerTarget
  /** The layer as the kernel last reported it; reloads are deferred meanwhile. */
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  refs: readonly PointRef[]
  delta: Point2
  handles: HandleMode
  /** Undo label for the run, already translated. */
  label: (count: number) => string
}

/** The optimistic geometry a run is showing. */
export interface NudgePreview {
  contours: readonly Contour[]
  anchors: readonly Anchor[]
}

/** What the runner reports back. */
export interface NudgeRunnerOptions {
  /** A run closed, from its idle timer or from any other action. */
  onEnd: () => void
  onError: (error: unknown) => void
}

export class NudgeRunner {
  private readonly options: NudgeRunnerOptions
  private coalescer: NudgeCoalescer | null = null
  private run: {
    base: readonly Contour[]
    anchors: readonly Anchor[]
    total: Point2
    refs: PointRef[]
  } | null = null

  constructor(options: NudgeRunnerOptions) {
    this.options = options
  }

  /** Whether a run is currently open. */
  get active(): boolean {
    return this.coalescer?.active === true
  }

  /** Applies one press and returns the geometry to draw. */
  press(input: NudgePress): NudgePreview {
    this.coalescer ??= new NudgeCoalescer({
      doc: input.target.doc,
      glyph: input.target.glyph,
      layer: input.target.layer,
      label: input.label,
      onEnd: () => this.close(),
      onError: this.options.onError,
    })
    this.coalescer.nudge(input.refs, input.delta, input.handles)

    // Accumulate against the layer the run started from: reloads are deferred
    // for its whole length, so that geometry stays put.
    const base = this.run?.base ?? input.contours
    const total = this.run
      ? {
          x: this.run.total.x + input.delta.x,
          y: this.run.total.y + input.delta.y,
        }
      : input.delta
    this.run = {
      base,
      anchors: this.run?.anchors ?? input.anchors,
      total,
      refs: movedPointRefs(base, input.refs, input.handles),
    }
    return {
      contours: translatePoints(base, this.run.refs, total.x, total.y),
      anchors: this.run.anchors,
    }
  }

  /** Finalises the run now: one undo entry for everything nudged so far. */
  async end(): Promise<void> {
    if (!this.coalescer?.active) return
    try {
      await this.coalescer.end()
    } catch (error) {
      this.options.onError(error)
    }
  }

  private close(): void {
    this.run = null
    this.options.onEnd()
  }
}
