import type { DocId } from '../api'

import { documentsWithEditParticipants } from './documentEditParticipants'
import { settleDocumentEdits } from './documentPersistence'

/**
 * Persists local drafts whenever this window stops being the active one.
 *
 * The draft registry is per window: a detached view holds the pending edit of
 * its own editors, and Save or Close in another window drains only the
 * registry it can see. Leaving a window is therefore the moment its drafts
 * have to reach the kernel — by the time the user saves in the workspace
 * window, the detached Font Info or feature editor must no longer be the only
 * place that edit exists.
 *
 * Failures are reported rather than thrown: this runs from a browser event
 * with nobody to reject to, and the draft stays pending for the next attempt.
 */
export function installDraftFlushOnWindowBlur(
  report: (error: unknown) => void,
  docs: () => readonly DocId[] = documentsWithEditParticipants,
): () => void {
  const flush = () => {
    for (const doc of docs()) {
      settleDocumentEdits(doc).catch(report)
    }
  }
  const onVisibility = () => {
    if (document.visibilityState === 'hidden') flush()
  }
  window.addEventListener('blur', flush)
  window.addEventListener('pagehide', flush)
  document.addEventListener('visibilitychange', onVisibility)
  return () => {
    window.removeEventListener('blur', flush)
    window.removeEventListener('pagehide', flush)
    document.removeEventListener('visibilitychange', onVisibility)
  }
}
