import { useCallback, useEffect, useState } from 'react'
import { call, onDocumentEvent, type DocId } from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { i18n } from '../../i18n'
import { errorNotice } from '../../i18n/errors'
import { useDocumentStore } from '../../stores/documentStore'
import { invalidateFontInfo } from '../../stores/fontInfoCache'
import {
  ProofLibraryError,
  readProofLibrary,
  writeProofLibrary,
  type ProofEntry,
} from './proofLibrary'

interface LoadedLibrary {
  doc: DocId | null
  key: string
  entries: ProofEntry[]
  error: string | null
}
interface OperationState {
  pending: number
  error: string | null
}

function message(error: unknown): string {
  return error instanceof ProofLibraryError
    ? i18n.t(`proof.library.errors.${error.code}`)
    : errorNotice(error, i18n.t.bind(i18n)).message
}

/** The saved project proofs and commands that create independent undo entries. */
export interface ProofLibraryResult {
  entries: ProofEntry[]
  loading: boolean
  saving: boolean
  error: string | null
  save: (entry: ProofEntry) => Promise<boolean>
  remove: (id: string) => Promise<boolean>
}

/**
 * Loads project-owned proofs, follows undo/redo and applies edits against fresh
 * font info inside the same barrier used by document save/history/close.
 */
export function useProofLibrary(doc: DocId | null): ProofLibraryResult {
  const generation = useDocumentStore((state) =>
    doc === null ? 0 : (state.byDoc[doc]?.generation ?? 0),
  )
  const [revision, setRevision] = useState(0)
  const key = `${doc ?? 'none'}:${generation}:${revision}`
  const [loaded, setLoaded] = useState<LoadedLibrary>({
    doc: null,
    key: '',
    entries: [],
    error: null,
  })
  const [operations, setOperations] = useState<Record<DocId, OperationState>>(
    {},
  )

  useEffect(() => {
    if (doc === null) return
    return onDocumentEvent((event) => {
      if (event.doc === doc && event.change.font_info) {
        setRevision((value) => value + 1)
      }
    })
  }, [doc])

  useEffect(() => {
    if (doc === null) return
    let cancelled = false
    void call('doc.font_info', { doc })
      .then((view) => {
        const entries = readProofLibrary(view.info.user_data)
        if (!cancelled) setLoaded({ doc, key, entries, error: null })
      })
      .catch((error: unknown) => {
        if (!cancelled)
          setLoaded((previous) => ({
            doc,
            key,
            entries: previous.doc === doc ? previous.entries : [],
            error: message(error),
          }))
      })
    return () => {
      cancelled = true
    }
  }, [doc, key])

  const mutate = useCallback(
    async (
      update: (entries: ProofEntry[]) => ProofEntry[],
      label: string,
    ): Promise<boolean> => {
      if (doc === null) return false
      setOperations((current) => ({
        ...current,
        [doc]: { pending: (current[doc]?.pending ?? 0) + 1, error: null },
      }))
      try {
        await withSettledDocument(doc, async () => {
          const view = await call('doc.font_info', { doc })
          const entries = update(readProofLibrary(view.info.user_data))
          const user_data = writeProofLibrary(view.info.user_data, entries)
          await call('doc.apply', {
            doc,
            command: { SetFontInfo: { info: { ...view.info, user_data } } },
            label,
          })
          invalidateFontInfo(doc)
        })
        setRevision((value) => value + 1)
        return true
      } catch (error) {
        setOperations((current) => ({
          ...current,
          [doc]: { pending: current[doc]?.pending ?? 1, error: message(error) },
        }))
        return false
      } finally {
        setOperations((current) => ({
          ...current,
          [doc]: {
            pending: Math.max(0, (current[doc]?.pending ?? 1) - 1),
            error: current[doc]?.error ?? null,
          },
        }))
      }
    },
    [doc],
  )

  const save = useCallback(
    (entry: ProofEntry) =>
      mutate((entries) => {
        const index = entries.findIndex((saved) => saved.id === entry.id)
        if (index < 0) return [...entries, entry]
        return entries.map((saved, position) =>
          position === index ? entry : saved,
        )
      }, i18n.t('proof.library.undoSave')),
    [mutate],
  )
  const remove = useCallback(
    (id: string) =>
      mutate(
        (entries) => entries.filter((entry) => entry.id !== id),
        i18n.t('proof.library.undoRemove'),
      ),
    [mutate],
  )

  const operation = doc === null ? undefined : operations[doc]
  return {
    entries: loaded.doc === doc ? loaded.entries : [],
    loading: doc !== null && loaded.key !== key,
    saving: (operation?.pending ?? 0) > 0,
    error: operation?.error ?? (loaded.doc === doc ? loaded.error : null),
    save,
    remove,
  }
}
