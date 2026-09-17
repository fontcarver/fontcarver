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

/** Closures that a replacement editor for the same document must not overtake. */
const documentEdits = new Map<DocId, Promise<void>>()
const documentEditRevisions = new Map<DocId, number>()

/** Identifies edits started or staged since a document handover was captured. */
export function documentEditRevision(doc: DocId): number {
  return documentEditRevisions.get(doc) ?? 0
}

/** Records new edit intent so navigation also drains work begun while waiting. */
export function noteDocumentEdit(doc: DocId): void {
  documentEditRevisions.set(doc, documentEditRevision(doc) + 1)
}

function completed(promise: Promise<unknown>): Promise<void> {
  return promise.then(
    () => undefined,
    () => undefined,
  )
}

/** The current edit handover for a document, if one is still running. */
export function pendingDocumentEdits(doc: DocId): Promise<void> | undefined {
  return documentEdits.get(doc)
}

/** Publishes an edit closure so replacement controllers and commands await it. */
export function holdDocumentEdits(
  doc: DocId,
  closing: Promise<unknown>,
): Promise<void> {
  const prior = documentEdits.get(doc)
  const combined = prior
    ? Promise.allSettled([prior, closing]).then(() => undefined)
    : completed(closing)
  const tracked = combined.finally(() => {
    if (documentEdits.get(doc) === tracked) documentEdits.delete(doc)
  })
  documentEdits.set(doc, tracked)
  return tracked
}

/** Runs a document command after any controller handover has completed. */
export async function afterDocumentEdits<T>(
  doc: DocId,
  run: () => Promise<T>,
): Promise<T> {
  for (;;) {
    const pending = documentEdits.get(doc)
    if (!pending) return run()
    await pending
  }
}

function combineBarriers(
  first: Promise<unknown> | undefined,
  second: Promise<unknown> | undefined,
): Promise<unknown> | undefined {
  if (!first) return second
  if (!second) return first
  return Promise.allSettled([first, second])
}

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
/** Everything optional about a gesture's session. */
export interface EditSessionOptions {
  scheduler?: Scheduler
  onError?: (error: unknown) => void
  /**
   * A gate `edit_begin` waits for.
   *
   * The kernel finalises whatever edit is open when a new one begins, so a
   * gesture that starts while another is still closing would invalidate the
   * older gesture's last, still-queued apply. Handing the previous gesture's
   * completion in here keeps the two in order without making the pointer
   * handler asynchronous.
   */
  after?: Promise<unknown>
}

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
  private errored = false
  private warned = false
  private firstError: unknown = null
  private readonly onError: (error: unknown) => void

  constructor(doc: DocId, label: string, options: EditSessionOptions = {}) {
    this.doc = doc
    noteDocumentEdit(doc)
    this.label = label
    this.scheduler = options.scheduler ?? frameScheduler
    this.onError = options.onError ?? (() => undefined)
    const gate = combineBarriers(options.after, pendingDocumentEdits(doc))
    this.token = gate
      ? gate.then(
          () => call('doc.edit_begin', { doc, label }),
          () => call('doc.edit_begin', { doc, label }),
        )
      : call('doc.edit_begin', { doc, label })
    // Through `report`, not `onError`: a failed `edit_begin` rejects the token
    // that every later step awaits, so without marking the gesture failed here
    // the first of those steps would report the same error a second time.
    this.chain = this.token
      .then(() => undefined)
      .catch((error: unknown) => {
        this.report(error)
      })
  }

  /**
   * Appends one kernel call to the gesture's chain.
   *
   * The chain never rejects: a failed call is reported and the following steps
   * still run, because whatever else happens the gesture has to be closed — an
   * open edit token would swallow the next gesture's undo entry.
   */
  private step(run: () => Promise<void>): Promise<void> {
    this.chain = this.chain.then(run).catch((error: unknown) => {
      this.report(error)
    })
    return this.chain
  }

  /**
   * Reports the first failure of the gesture and swallows the rest.
   *
   * Every later step re-awaits the same rejected token, so a document the
   * kernel has lost would otherwise raise one toast per queued call.
   */
  private report(error: unknown): void {
    if (!this.errored) {
      this.errored = true
      this.firstError = error
      this.onError(error)
      return
    }
    // The usual repeat is the same rejected token surfacing in every queued
    // step, which is not worth a second toast. A genuinely different failure
    // still deserves a trace, once.
    if (error === this.firstError || this.warned) return
    this.warned = true
    console.warn('editing gesture failed again after the first error:', error)
  }

  /** Whether at least one command reached the kernel, i.e. an undo entry exists. */
  get dirty(): boolean {
    return this.applied
  }

  /**
   * Whether any call in this gesture failed. The document may then hold part
   * of the gesture, so the caller has to refetch rather than trust its preview.
   */
  get failed(): boolean {
    return this.errored
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
    return this.step(async () => {
      const token = await this.token
      await call('doc.edit_apply', { doc: this.doc, token, command })
      this.applied = true
    })
  }

  /** Sends everything queued and finalises the gesture into one undo entry. */
  async end(): Promise<void> {
    if (this.closed) return this.chain
    void this.flush()
    this.closed = true
    return this.step(async () => {
      const token = await this.token
      await call('doc.edit_end', { doc: this.doc, token })
    })
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
    if (!this.closed) {
      this.closed = true
      this.step(async () => {
        const token = await this.token
        await call('doc.edit_end', { doc: this.doc, token })
      })
    }
    // Whether an undo entry exists is decided *after* the calls already on the
    // wire have answered: Escape normally arrives while an apply is in flight,
    // and reading the flag now would conclude that nothing had been applied.
    return this.step(async () => {
      if (!this.applied) return
      await call('doc.undo', { doc: this.doc })
    })
  }
}

/** Applies one command as its own undo entry. */
export async function applyCommand(
  doc: DocId,
  command: Command,
  label: string,
): Promise<void> {
  await afterDocumentEdits(doc, () =>
    call('doc.apply', { doc, command, label }).then(() => undefined),
  )
}
