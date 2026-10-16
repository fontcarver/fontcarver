import type { DocId } from '../api'
import {
  documentEditRevision,
  holdDocumentEdits,
  noteDocumentEdit,
} from './editSession'

/** A document editor whose local state must survive navigation and history. */
export interface DocumentEditParticipant {
  /**
   * Persist the pending draft, finishing IME composition first. Validation
   * diagnostics may remain, but persistence errors must reject this promise.
   * Call the kernel directly: awaiting the document handover from inside its
   * own flush would deadlock. Flushes run after the outline controller settles.
   */
  flush: () => Promise<void>
  /** Enables document Undo before a local draft has reached the kernel. */
  hasPendingEdits?: () => boolean
  /** Temporarily reject user edits while the document is actually closing. */
  setSuspended?: (suspended: boolean) => void
}

interface Registration {
  participant: DocumentEditParticipant
  pending?: Promise<void>
}

const participants = new Map<DocId, Set<Registration>>()
const draining = new Map<DocId, Promise<void>>()
const suspensions = new Map<DocId, number>()

/** Whether this document is rejecting new local input during final close. */
export function documentEditingSuspended(doc: DocId | undefined): boolean {
  return doc !== undefined && suspensions.has(doc)
}

/**
 * Registers an editor independently of the canvas controller. Call
 * noteDocumentEdit(doc) on each local mutation, including changes during a
 * flush; hydration does not count as an edit. Dispose after a successful
 * handover has persisted the draft, never to abandon pending work.
 */
export function registerDocumentEditParticipant(
  doc: DocId,
  participant: DocumentEditParticipant,
): () => void {
  const entry: Registration = { participant }
  const entries = participants.get(doc) ?? new Set<Registration>()
  entries.add(entry)
  participants.set(doc, entries)
  if (suspensions.has(doc)) participant.setSuspended?.(true)
  noteDocumentEdit(doc)
  return () => {
    if (!entries.delete(entry)) return
    if (entries.size === 0) participants.delete(doc)
    noteDocumentEdit(doc)
  }
}

/**
 * Makes mounted editors read-only through close's final read and transport
 * call. Nested closers share the lock; failures/prompts restore input.
 */
export function suspendDocumentEditParticipants(doc: DocId): () => void {
  const count = suspensions.get(doc) ?? 0
  suspensions.set(doc, count + 1)
  if (count === 0)
    for (const { participant } of participants.get(doc) ?? [])
      participant.setSuspended?.(true)
  let released = false
  return () => {
    if (released) return
    released = true
    const remaining = (suspensions.get(doc) ?? 1) - 1
    if (remaining > 0) suspensions.set(doc, remaining)
    else {
      suspensions.delete(doc)
      for (const { participant } of participants.get(doc) ?? [])
        participant.setSuspended?.(false)
    }
  }
}

/** Documents that currently have a registered editor in this window. */
export function documentsWithEditParticipants(): readonly DocId[] {
  return [...participants.keys()]
}

/** Whether a registered editor has a local draft that document Undo can flush. */
export function hasPendingDocumentDraft(doc: DocId): boolean {
  return [...(participants.get(doc) ?? [])].some(
    ({ participant }) => participant.hasPendingEdits?.() ?? false,
  )
}

function flush(entry: Registration): Promise<void> {
  if (entry.pending) return entry.pending
  const pending = Promise.resolve()
    .then(() => entry.participant.flush())
    .finally(() => {
      if (entry.pending === pending) entry.pending = undefined
    })
  entry.pending = pending
  return pending
}

/**
 * Persists registered drafts in order so only one document transaction opens
 * at a time. A local edit or replacement editor arriving during an await
 * causes another pass. Rejections reach the navigation/persistence caller.
 */
async function drain(doc: DocId): Promise<void> {
  for (;;) {
    const revision = documentEditRevision(doc)
    for (const entry of [...(participants.get(doc) ?? [])]) {
      if (participants.get(doc)?.has(entry)) await flush(entry)
    }
    if (documentEditRevision(doc) === revision) return
  }
}

/** Joins an existing drain so concurrent handovers share one transaction owner. */
export function flushDocumentEditParticipants(doc: DocId): Promise<void> {
  const current = draining.get(doc)
  if (current) return current
  const pending = drain(doc).finally(() => {
    if (draining.get(doc) === pending) draining.delete(doc)
  })
  draining.set(doc, pending)
  // Order later gestures/commands too, while returning the original rejecting
  // promise to the caller that must keep its view intact on persistence error.
  void holdDocumentEdits(doc, pending)
  return pending
}
