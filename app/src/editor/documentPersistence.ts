import type { DocId } from '../api'
import { activeEditor } from './activeEditor'
import { holdDocumentEdits, pendingDocumentEdits } from './editSession'

/** Finishes owned edits and keeps subsequent edits behind a persistence call. */
export function withSettledDocument<T>(
  doc: DocId,
  run: () => Promise<T>,
): Promise<T> {
  const editor = activeEditor()
  const previous = pendingDocumentEdits(doc)
  // Capture old work before publishing this operation: tool completion must
  // never wait for the persistence call that is waiting for that tool.
  const prepared =
    editor?.documentId === doc ? editor.prepareTransition() : undefined
  const operation = Promise.all([previous, prepared]).then(run)
  void holdDocumentEdits(doc, operation)
  return operation
}
