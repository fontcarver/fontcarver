/**
 * The kernel side of one editing gesture.
 *
 * A gesture is `doc.edit_begin` → N × `doc.edit_apply` → `doc.edit_end`, which
 * the kernel coalesces into exactly one undo entry carrying the label given at
 * the beginning. Applies are throttled to animation frames: the canvas follows
 * the pointer from a local copy of the outline, so the kernel never has to
 * keep up with the pointer and no round trip is on the critical path.
 *
 * `MovePoints` and `TransformSelection` translate the *current* stored state,
 * so each frame sends the delta since the previous apply. Their sum is the
 * delta of the whole gesture, which is what the user sees on screen.
 */

import { call, type Command, type DocId, type EditTokenId } from '../api'

/** How queued applies are deferred; injected so tests can step frames. */
export interface Scheduler {
  request: (callback: () => void) => number
  cancel: (handle: number) => void
}

/** The animation-frame scheduler used in the browser. */
export const frameScheduler: Scheduler = {
  request: (callback) =>
    typeof requestAnimationFrame === 'function'
      ? requestAnimationFrame(() => callback())
      : (setTimeout(callback, 16) as unknown as number),
  cancel: (handle) => {
    if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(handle)
    else clearTimeout(handle)
  },
}

/** A scheduler that runs the callback synchronously; used by tests. */
export const immediateScheduler: Scheduler = {
  request: (callback) => {
    callback()
    return 0
  },
  cancel: () => undefined,
}

/**
 * One live `edit_begin`/`edit_apply`/`edit_end` gesture.
 *
 * Every kernel call is chained onto the previous one so the transport sees the
 * protocol in order even though callers are synchronous pointer handlers.
 */
export class EditSession {
  readonly doc: DocId
  readonly label: string

  private readonly scheduler: Scheduler
  private token: Promise<EditTokenId>
  private queued: (() => Command | null) | null = null
  private frame: number | null = null
  private chain: Promise<void>
  private applied = false
  private closed = false

  constructor(doc: DocId, label: string, scheduler: Scheduler = frameScheduler) {
    this.doc = doc
    this.label = label
    this.scheduler = scheduler
    this.token = call('doc.edit_begin', { doc, label })
    this.chain = this.token.then(() => undefined)
  }

  /** Whether at least one command reached the kernel, i.e. an undo entry exists. */
  get dirty(): boolean {
    return this.applied
  }

  /** Resolves once `edit_begin` has answered. */
  async ready(): Promise<void> {
    await this.token
  }

  /**
   * Queues a command for the next animation frame, replacing anything queued
   * but not yet sent: a gesture only ever needs its most recent state. The
   * command is *built* when the frame runs, so a builder that reports the step
   * since the last apply sees every intermediate move exactly once.
   */
  queue(build: () => Command | null): void {
    if (this.closed) return
    this.queued = build
    if (this.frame !== null) return
    this.frame = this.scheduler.request(() => {
      this.frame = null
      void this.flush()
    })
  }

  /** Sends whatever is queued right now. */
  flush(): Promise<void> {
    if (this.frame !== null) {
      this.scheduler.cancel(this.frame)
      this.frame = null
    }
    const build = this.queued
    this.queued = null
    if (build === null || this.closed) return this.chain
    const command = build()
    if (command === null) return this.chain
    this.chain = this.chain.then(async () => {
      const token = await this.token
      await call('doc.edit_apply', { doc: this.doc, token, command })
      this.applied = true
    })
    return this.chain
  }

  /** Sends everything queued and finalises the gesture into one undo entry. */
  async end(): Promise<void> {
    if (this.closed) return this.chain
    await this.flush()
    this.closed = true
    this.chain = this.chain.then(async () => {
      const token = await this.token
      await call('doc.edit_end', { doc: this.doc, token })
    })
    return this.chain
  }

  /**
   * Abandons the gesture: the queued state is dropped, the session is closed
   * and, when something was already applied, the resulting undo entry is
   * undone. The kernel has no "roll back a live edit" call — closing and
   * undoing is the documented way to cancel, and because the whole gesture is
   * a single coalesced entry one `doc.undo` removes all of it.
   */
  async cancel(): Promise<void> {
    this.queued = null
    if (this.frame !== null) {
      this.scheduler.cancel(this.frame)
      this.frame = null
    }
    const wasDirty = this.applied
    if (!this.closed) {
      this.closed = true
      this.chain = this.chain.then(async () => {
        const token = await this.token
        await call('doc.edit_end', { doc: this.doc, token })
      })
    }
    if (wasDirty)
      this.chain = this.chain.then(async () => {
        await call('doc.undo', { doc: this.doc })
      })
    return this.chain
  }
}

/** Applies one command as its own undo entry. */
export async function applyCommand(
  doc: DocId,
  command: Command,
  label: string,
): Promise<void> {
  await call('doc.apply', { doc, command, label })
}
