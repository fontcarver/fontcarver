/**
 * Arrow-key nudging.
 *
 * A run of arrow presses is one edit: the first key opens a gesture, every key
 * applies into it, and 500 ms of silence — or any other action — closes it, so
 * "tap left twelve times" is one undo entry rather than twelve (design spec
 * §4.3, brief step 1). The step is 1 unit, 10 with Shift and 100 with Ctrl,
 * and Alt nudges the node without its handles.
 */

import type { Command, DocId, HandleMode, PointRef } from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import { EditSession, frameScheduler, type Scheduler } from '../editSession'

import type { Modifiers } from './drag'

/** Idle time after which a run of nudges becomes one undo entry, in ms. */
export const NUDGE_TIMEOUT = 500

/** Nudge distances in font units, design spec §4.3. */
export const NUDGE_STEPS = { normal: 1, shift: 10, mod: 100 } as const

/** The distance one arrow press moves the selection. */
export function nudgeStep(modifiers: Modifiers): number {
  if (modifiers.mod) return NUDGE_STEPS.mod
  if (modifiers.shift) return NUDGE_STEPS.shift
  return NUDGE_STEPS.normal
}

/** What a nudge run needs to know. */
export interface NudgeContext {
  doc: DocId
  glyph: string
  layer: string
  /** Undo label for the run, already translated. */
  label: (count: number) => string
  scheduler?: Scheduler
  /** Idle timeout; injected by tests. */
  timeout?: number
  /**
   * Called when a run closes, including from the idle timer. Without it the
   * owner cannot know that the gesture is over, because the timer fires inside
   * this object rather than in response to any input.
   */
  onEnd?: () => void
  /** Where a failed kernel call is reported. */
  onError?: (error: unknown) => void
}

/**
 * Collects arrow presses into one coalesced edit.
 *
 * The pending delta is accumulated locally and the *step since the last apply*
 * is what reaches the kernel, because `MovePoints` translates the current
 * state. Applies still ride the animation frame, so holding an arrow key down
 * cannot outrun the transport.
 */
export class NudgeCoalescer {
  private readonly context: NudgeContext
  private session: EditSession | null = null
  private signature = ''
  private total: Point2 = { x: 0, y: 0 }
  private sent: Point2 = { x: 0, y: 0 }
  private refs: PointRef[] = []
  private handles: HandleMode = 'WithHandles'
  private timer: ReturnType<typeof setTimeout> | null = null

  constructor(context: NudgeContext) {
    this.context = context
  }

  /** Whether a run is currently open. */
  get active(): boolean {
    return this.session !== null
  }

  /** Adds one arrow press. */
  nudge(refs: readonly PointRef[], delta: Point2, handles: HandleMode): void {
    if (refs.length === 0) return
    const signature = `${handles}|${refs
      .map((ref) => `${ref.contour}:${ref.point}`)
      .join(',')}`
    if (this.session && signature !== this.signature) void this.end()
    if (!this.session) {
      this.signature = signature
      this.refs = [...refs]
      this.handles = handles
      this.total = { x: 0, y: 0 }
      this.sent = { x: 0, y: 0 }
      this.session = new EditSession(
        this.context.doc,
        this.context.label(refs.length),
        this.context.scheduler ?? frameScheduler,
        this.context.onError,
      )
    }
    this.total = { x: this.total.x + delta.x, y: this.total.y + delta.y }
    this.session.queue(() => this.command())
    this.restartTimer()
  }

  /** Sends whatever is queued right now; used by tests and by a flush. */
  flush(): Promise<void> {
    return this.session?.flush() ?? Promise.resolve()
  }

  /** Finalises the run: one undo entry for everything nudged so far. */
  async end(): Promise<void> {
    if (this.timer !== null) {
      clearTimeout(this.timer)
      this.timer = null
    }
    const session = this.session
    this.session = null
    this.signature = ''
    if (!session) return
    // The owner is told synchronously, before the round trip: it has to drop
    // its optimistic preview and re-enable reloads whether or not the kernel
    // answers, and a run started meanwhile must not be closed by this one.
    this.context.onEnd?.()
    await session.end()
  }

  private restartTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = setTimeout(() => {
      this.timer = null
      void this.end()
    }, this.context.timeout ?? NUDGE_TIMEOUT)
  }

  private command(): Command | null {
    const step = {
      x: this.total.x - this.sent.x,
      y: this.total.y - this.sent.y,
    }
    if (step.x === 0 && step.y === 0) return null
    this.sent = { ...this.total }
    return {
      MovePoints: {
        glyph: this.context.glyph,
        layer: this.context.layer,
        points: this.refs,
        delta: [step.x, step.y],
        handles: this.handles,
      },
    }
  }
}
