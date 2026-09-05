/**
 * The lifetime of an arrow-key nudge run.
 *
 * A run is one coalesced edit — one undo entry — that stays open for 500 ms of
 * silence, and while it is open the canvas draws from a local copy so the very
 * first key press moves the outline. Both halves of that belong together:
 * whoever owns the coalescer also owns the accumulated preview, and closing
 * one without the other is what left the editor stuck in an earlier round.
 */

import type {
  Anchor,
  ComponentView,
  Contour,
  HandleMode,
  PointRef,
} from '../api'
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
  components: readonly ComponentView[]
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
  /** Untouched: a nudge sends `MovePoints`, which moves no component. */
  components: readonly ComponentView[]
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
    components: readonly ComponentView[]
    total: Point2
    refs: PointRef[]
  } | null = null
  private signature = ''
  /**
   * What a run that closed on a selection change left behind.
   *
   * Reloads are deferred for a run's whole length, so the outline the next
   * press is handed is the one fetched *before* the previous run. Its result is
   * carried forward as the next run's base until the reload lands, or the
   * points the earlier run moved would draw back at their old positions.
   */
  private carried: readonly Contour[] | null = null
  /** Identifies the open run, so a late `onEnd` cannot close a newer one. */
  private generation = 0
  /** The run that is closing, which the next one's `edit_begin` waits for. */
  private closing: Promise<void> | undefined

  constructor(options: NudgeRunnerOptions) {
    this.options = options
  }

  /** Whether a run is currently open. */
  get active(): boolean {
    return this.coalescer?.active === true
  }

  /** Applies one press and returns the geometry to draw. */
  press(input: NudgePress): NudgePreview {
    const signature = `${input.handles}|${input.refs
      .map((ref) => `${ref.contour}:${ref.point}`)
      .join(',')}`
    // A coalescer holds exactly one run: it is replaced when the selection
    // changes and when the previous run has closed, so every run gets its own
    // generation and its own gate.
    const stale =
      this.coalescer !== null &&
      (!this.coalescer.active || signature !== this.signature)
    if (stale) {
      if (this.run)
        // Reloads were deferred for the closing run's whole length, so its
        // result is what the next one has to accumulate from.
        this.carried = translatePoints(
          this.run.base,
          this.run.refs,
          this.run.total.x,
          this.run.total.y,
        )
      this.run = null
      this.closing = this.end()
      this.coalescer = null
    }
    this.signature = signature

    if (!this.coalescer) {
      this.generation += 1
      const generation = this.generation
      this.coalescer = new NudgeCoalescer({
        doc: input.target.doc,
        glyph: input.target.glyph,
        layer: input.target.layer,
        label: input.label,
        after: this.closing,
        onEnd: () => this.close(generation),
        onError: this.options.onError,
      })
    }
    this.coalescer.nudge(input.refs, input.delta, input.handles)

    // Accumulate against the layer the run started from: reloads are deferred
    // for its whole length, so that geometry stays put.
    const base = this.run?.base ?? this.carried ?? input.contours
    const total = this.run
      ? {
          x: this.run.total.x + input.delta.x,
          y: this.run.total.y + input.delta.y,
        }
      : input.delta
    this.run = {
      base,
      anchors: this.run?.anchors ?? input.anchors,
      components: this.run?.components ?? input.components,
      total,
      refs: movedPointRefs(base, input.refs, input.handles),
    }
    return {
      contours: translatePoints(base, this.run.refs, total.x, total.y),
      anchors: this.run.anchors,
      components: this.run.components,
    }
  }

  /** Finalises the run now: one undo entry for everything nudged so far. */
  async end(): Promise<void> {
    const coalescer = this.coalescer
    if (!coalescer?.active) return
    try {
      await coalescer.end()
    } catch (error) {
      this.options.onError(error)
    }
  }

  private close(generation: number): void {
    // A run started while this one was closing owns the preview now.
    if (generation !== this.generation) return
    this.run = null
    this.carried = null
    this.closing = undefined
    this.options.onEnd()
  }
}
