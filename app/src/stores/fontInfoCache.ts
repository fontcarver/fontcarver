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

  const request = call('doc.font_info', { doc })
    .then((info) => {
      cache.set(doc, info)
      return info
    })
    .finally(() => inFlight.delete(doc))
  inFlight.set(doc, request)
  return request
}

/** Drops one document's cached font info so the next read refetches it. */
export function forgetFontInfo(doc: DocId): void {
  cache.delete(doc)
}

onDocumentForgotten(forgetFontInfo)

/** What `useFontInfo` reports. */
export interface FontInfoResult {
  info: FontInfoView | null
  error: string | null
  loading: boolean
}

/**
 * The font info of one document. `reloadToken` is bumped by the caller when a
 * document event reports that font-wide or master data changed.
 *
 * The cache is read during render and only the *resolved* request writes state,
 * so a document whose info is already cached renders it on the first pass.
 */
export function useFontInfo(
  doc: DocId | null,
  reloadToken = 0,
): FontInfoResult {
  const key = `${doc ?? 'none'}:${reloadToken}`
  const [loaded, setLoaded] = useState<{
    key: string
    info: FontInfoView | null
    error: string | null
  }>({ key: '', info: null, error: null })

  useEffect(() => {
    if (doc === null) return
    if (reloadToken > 0) forgetFontInfo(doc)
    let cancelled = false
    void loadFontInfo(doc)
      .then((info) => {
        if (!cancelled) setLoaded({ key, info, error: null })
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setLoaded({
          key,
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
  const cached = cachedFontInfo(doc)
  return {
    info: cached ?? null,
    error: null,
    loading: cached === undefined,
  }
}
