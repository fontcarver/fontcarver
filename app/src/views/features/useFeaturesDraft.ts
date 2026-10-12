import { useEffect, useMemo, useSyncExternalStore } from 'react'
import type { DocId } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { FeatureDraftSession } from './FeatureDraftSession'

/** Attach the draft owner to document save, history and navigation barriers. */
export function useFeaturesDraft(
  doc: DocId,
  generation: number,
  master: string | null,
) {
  const session = useMemo(() => new FeatureDraftSession(doc), [doc])
  const snapshot = useSyncExternalStore(session.subscribe, session.getSnapshot)
  useEffect(
    () =>
      registerDocumentEditParticipant(doc, {
        flush: session.flush,
        hasPendingEdits: session.hasPendingEdits,
        setSuspended: session.setSuspended,
      }),
    [doc, session],
  )
  useEffect(() => {
    session.invalidate(master)
    void session.reload().catch(() => undefined)
  }, [session, generation, master])
  useEffect(() => {
    if (!snapshot.pending || snapshot.saving) return
    const timer = setTimeout(
      () => void session.flush().catch(() => undefined),
      700,
    )
    return () => clearTimeout(timer)
  }, [
    session,
    snapshot.source,
    snapshot.settings,
    snapshot.pending,
    snapshot.saving,
  ])
  return { session, ...snapshot }
}
