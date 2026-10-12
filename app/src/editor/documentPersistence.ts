import type { DocId } from '../api'
import { activeEditor } from './activeEditor'
import { flushDocumentEditParticipants } from './documentEditParticipants'
import {
  documentEditRevision,
  holdDocumentEdits,
  pendingDocumentEdits,
} from './editSession'

/**
 * Drains the controller and registered drafts before navigation. Captures the
 * current controller so a delayed handover never finishes a replacement view.
 * Unlike the gesture queue, participant persistence failures must reject.
 */
export async function settleDocumentEdits(
  doc: DocId,
  isCurrent: () => boolean = () => true,
): Promise<void> {
  await prepareDocumentEdits(doc, isCurrent)
}

async function prepareDocumentEdits(
  doc: DocId,
  isCurrent: () => boolean,
): Promise<number | undefined> {
  const editor = activeEditor()
  for (;;) {
    if (!isCurrent()) return
    const revision = documentEditRevision(doc)
    const previous = pendingDocumentEdits(doc)
    const prepared =
      editor?.documentId === doc ? editor.prepareTransition() : undefined
    await Promise.all([previous, prepared])
    if (!isCurrent()) return
    // A gesture begun during the old close must finish before a draft command
    // can open its own kernel transaction and implicitly finalize that gesture.
    if (documentEditRevision(doc) !== revision) continue
    await flushDocumentEditParticipants(doc)
    for (;;) {
      const pending = pendingDocumentEdits(doc)
      if (!pending) break
      await pending
    }
    if (documentEditRevision(doc) === revision) return revision
  }
}

const persistenceOperations = new Map<DocId, Promise<unknown>>()

async function persist<T>(doc: DocId, run: () => Promise<T>): Promise<T> {
  for (;;) {
    const revision = await prepareDocumentEdits(doc, () => true)
    // Close the microtask gap between draining and beginning persistence.
    if (documentEditRevision(doc) !== revision) continue
    const operation = Promise.resolve().then(run)
    void holdDocumentEdits(doc, operation)
    return operation
  }
}

/**
 * Finishes edits including those staged during preparation, then gates later
 * gestures behind persistence. A separate operation queue orders concurrent
 * save/history/close requests without making preparation wait on itself.
 */
export function withSettledDocument<T>(
  doc: DocId,
  run: () => Promise<T>,
): Promise<T> {
  const previous = persistenceOperations.get(doc)
  const starting = previous
    ? previous.catch(() => undefined).then(() => persist(doc, run))
    : persist(doc, run)
  const operation = starting.finally(() => {
    if (persistenceOperations.get(doc) === operation)
      persistenceOperations.delete(doc)
  })
  persistenceOperations.set(doc, operation)
  return operation
}
