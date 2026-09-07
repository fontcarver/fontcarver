import type { DocId } from '../api'
import { holdDocumentEdits, pendingDocumentEdits } from './editSession'

/** Orders local closures, tool commits and replacement document editors. */
export class EditorHandover {
  pending: Promise<void> | undefined
  externalSettler: (() => Promise<void>) | null = null
  finishing = false
  beforeTools: Promise<void> | null = null
  private transition: Promise<void> | null = null
  private applications: Promise<void>[] = []

  trackApplication(applying: Promise<void>): void {
    if (this.finishing) this.applications.push(applying)
  }

  gate(closing: Promise<unknown>, doc: DocId | undefined): Promise<void> {
    const tracked = closing
      .then(
        () => undefined,
        () => undefined,
      )
      .finally(() => {
        if (this.pending === tracked) this.pending = undefined
      })
    this.pending = tracked
    return doc === undefined ? tracked : holdDocumentEdits(doc, tracked)
  }

  /** Tool finish may call a normal command; avoid recursively finishing it. */
  prepare(
    doc: DocId | undefined,
    settle: () => Promise<void>,
    finishTools: () => void,
    settleCore: () => Promise<void>,
  ): Promise<void> {
    if (this.transition) return this.transition
    // Capture everything before tools finish. A future external EditSession
    // can then wait on this promise without its own close entering the snapshot.
    const previous = doc === undefined ? undefined : pendingDocumentEdits(doc)
    const before = Promise.allSettled([previous, settle()]).then(
      () => undefined,
    )
    this.finishing = true
    this.beforeTools = before
    this.applications = []
    let closing: Promise<void>
    try {
      finishTools()
      closing = settleCore()
    } finally {
      this.finishing = false
      this.beforeTools = null
    }
    const snapshot = [before, closing, ...this.applications]
    this.applications = []
    const tracked = Promise.allSettled(snapshot)
      .then(() => undefined)
      .finally(() => {
        if (this.transition === tracked) this.transition = null
      })
    this.transition = tracked
    return tracked
  }
}
