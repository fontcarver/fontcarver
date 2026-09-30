/**
 * Where a rejected glyph query is reported.
 *
 * A malformed search — an unfinished regular expression, a smart expression with
 * a typo — is not a transport failure and does not belong in a toast: the person
 * is typing, and the answer belongs next to the field they are typing in. The
 * grid's pager routes such a rejection here, the search field and the
 * smart-filter panel read it, and anything else still goes to `reportError`.
 */

import { create } from 'zustand'

import type { DocId } from '../../api'
import { ApiCallError } from '../../transport'

/** Which control a rejected query belongs to. */
export type QueryErrorKind = 'search' | 'smartFilter'

/** A rejected query, as the interface shows it. */
export interface QueryError {
  kind: QueryErrorKind
  /** The kernel's own wording, which names the offending part of the input. */
  message: string
}

/** Recognises the two query rejections the kernel reports for a listing. */
export function classifyQueryError(error: unknown): QueryError | null {
  if (!(error instanceof ApiCallError) || error.code !== 'invalid_params') {
    return null
  }
  const message = error.message
  if (/glyph search/i.test(message)) return { kind: 'search', message }
  if (/smart[- ]filter/i.test(message)) return { kind: 'smartFilter', message }
  return null
}

interface QueryErrorState {
  byDoc: Readonly<Record<DocId, QueryError>>
  /** Records a rejection, or clears the document's error when it is not one. */
  report: (doc: DocId, error: unknown) => boolean
  clear: (doc: DocId) => void
}

export const useQueryErrors = create<QueryErrorState>((set, get) => ({
  byDoc: {},
  report: (doc, error) => {
    const classified = classifyQueryError(error)
    if (!classified) return false
    const current = get().byDoc[doc]
    if (
      current?.kind === classified.kind &&
      current.message === classified.message
    ) {
      return true
    }
    set((state) => ({ byDoc: { ...state.byDoc, [doc]: classified } }))
    return true
  },
  clear: (doc) => {
    if (get().byDoc[doc] === undefined) return
    set((state) => {
      const byDoc = { ...state.byDoc }
      delete byDoc[doc]
      return { byDoc }
    })
  },
}))

/** The current rejection of one kind for one document, or `null`. */
export function selectQueryError(doc: DocId, kind: QueryErrorKind) {
  return (state: QueryErrorState): string | null => {
    const error = state.byDoc[doc]
    return error?.kind === kind ? error.message : null
  }
}
