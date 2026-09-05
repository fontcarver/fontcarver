/**
 * Per-document cache of `doc.font_info`.
 *
 * The response carries font-wide metadata plus every master record, which the
 * glyph grid (for the em box), the glyph editor (for vertical metrics, zones
 * and guides) and the Font Info tab all need. Fetching it once per document
 * and sharing the result keeps a font with 65k glyphs to a single request.
 */

import { useEffect, useState } from 'react'

import { call, type DocId, type FontInfoView } from '../api'

import { onDocumentForgotten } from './documentStore'

const cache = new Map<DocId, FontInfoView>()
const inFlight = new Map<DocId, Promise<FontInfoView>>()
const invalidationListeners = new Set<(doc: DocId) => void>()
/**
 * Bumped every time a document's font info is dropped. A request stamps the
 * revision it was issued for, so an answer computed before an edit is discarded
 * instead of being written back over the state that edit produced.
 */
const revisions = new Map<DocId, number>()

function revisionOf(doc: DocId): number {
  return revisions.get(doc) ?? 0
}

/** The cached font info of one document, when it has already been loaded. */
export function cachedFontInfo(doc: DocId): FontInfoView | undefined {
  return cache.get(doc)
}

/** Loads a document's font info, reusing an in-flight request. */
export function loadFontInfo(doc: DocId): Promise<FontInfoView> {
  const cached = cache.get(doc)
  if (cached) return Promise.resolve(cached)
  const pending = inFlight.get(doc)
  if (pending) return pending

  const issued = revisionOf(doc)
  /** Whether this request is still the one the cache is waiting for. */
  const current = () => revisionOf(doc) === issued
  const request = call('doc.font_info', { doc })
    .then((info) => {
      // An edit that landed while this was in flight already dropped the entry
      // and started a newer request; caching now would reinstate stale data,
      // and clearing `inFlight` would evict that newer request.
      if (current()) {
        cache.set(doc, info)
        inFlight.delete(doc)
      }
      return info
    })
    .catch((error: unknown) => {
      if (current()) inFlight.delete(doc)
      throw error
    })
  inFlight.set(doc, request)
  return request
}

/**
 * Drops one document's cached font info so the next read refetches it, and
 * retires whatever request is in flight for it.
 */
export function forgetFontInfo(doc: DocId): void {
  revisions.set(doc, revisionOf(doc) + 1)
  inFlight.delete(doc)
  cache.delete(doc)
}

/**
 * Subscribes to font-info invalidation; returns an unsubscribe function.
 *
 * The grid's em box, the Font Info tab and the glyph editor all read the same
 * response, so they all have to hear when an edit made it stale — whichever of
 * them happens to be mounted.
 */
export function onFontInfoInvalidated(
  listener: (doc: DocId) => void,
): () => void {
  invalidationListeners.add(listener)
  return () => {
    invalidationListeners.delete(listener)
  }
}

/** Drops the cached font info of one document and asks every reader to refetch. */
export function invalidateFontInfo(doc: DocId): void {
  forgetFontInfo(doc)
  for (const listener of [...invalidationListeners]) listener(doc)
}

onDocumentForgotten(forgetFontInfo)

/** What `useFontInfo` reports. */
export interface FontInfoResult {
  info: FontInfoView | null
  error: string | null
  loading: boolean
}

/**
 * The font info of one document.
 *
 * Refetches whenever {@link invalidateFontInfo} reports the response stale;
 * `reloadToken` does the same for callers that track staleness themselves (the
 * glyph tab follows its own document generation).
 *
 * The cache is read during render and only the *resolved* request writes state,
 * so a document whose info is already cached renders it on the first pass, and
 * a refetch keeps showing the previous answer instead of flashing an empty tab.
 */
export function useFontInfo(
  doc: DocId | null,
  reloadToken = 0,
): FontInfoResult {
  const [revision, setRevision] = useState(0)
  const key = `${doc ?? 'none'}:${reloadToken}:${revision}`
  const [loaded, setLoaded] = useState<{
    key: string
    doc: DocId | null
    info: FontInfoView | null
    error: string | null
  }>({ key: '', doc: null, info: null, error: null })

  useEffect(() => {
    if (doc === null) return
    return onFontInfoInvalidated((invalidated) => {
      if (invalidated === doc) setRevision((value) => value + 1)
    })
  }, [doc])

  useEffect(() => {
    if (doc === null) return
    if (reloadToken > 0) forgetFontInfo(doc)
    let cancelled = false
    void loadFontInfo(doc)
      .then((info) => {
        if (!cancelled) setLoaded({ key, doc, info, error: null })
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setLoaded({
          key,
          doc,
          info: null,
          error: error instanceof Error ? error.message : String(error),
        })
      })
    return () => {
      cancelled = true
    }
  }, [doc, key, reloadToken])

  if (doc === null) return { info: null, error: null, loading: false }
  if (loaded.key === key)
    return { info: loaded.info, error: loaded.error, loading: false }
  // A refetch in flight: the freshly invalidated cache is empty, so fall back
  // to the answer already on screen rather than tearing the panel down.
  const shown = cachedFontInfo(doc) ?? (loaded.doc === doc ? loaded.info : null)
  return { info: shown, error: null, loading: shown === null }
}
