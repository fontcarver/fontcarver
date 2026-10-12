import { useEffect, useState } from 'react'

import { call, type DocId, type GlyphCell, type MasterId } from '../api'
import { emptyFilter } from '../stores/documentStore'

/** The bounded result of querying glyph names in the active document. */
export interface PaletteGlyphSearch {
  items: readonly GlyphCell[]
  total: number
  loading: boolean
  error: string | null
}

const EMPTY_SEARCH: PaletteGlyphSearch = {
  items: [],
  total: 0,
  loading: false,
  error: null,
}

/** Debounces metadata-only search and prevents old requests replacing new matches. */
export function usePaletteGlyphSearch(
  doc: DocId | null,
  master: MasterId | null,
  generation: number,
  query: string,
  enabled: boolean,
): PaletteGlyphSearch {
  const key = JSON.stringify([doc, master, generation, query.trim(), enabled])
  const [result, setResult] = useState<
    (PaletteGlyphSearch & { key: string }) | null
  >(null)

  useEffect(() => {
    if (!enabled || doc === null) return
    let current = true
    const timer = setTimeout(() => {
      void call('glyphs.list', {
        doc,
        master,
        filter: { ...emptyFilter(), query: query.trim() || null },
        sort: 'Name',
        offset: 0,
        limit: 50,
        include_paths: false,
      }).then(
        (page) => {
          if (current) setResult({ key, ...page, loading: false, error: null })
        },
        (error: unknown) => {
          if (current)
            setResult({
              key,
              items: [],
              total: 0,
              loading: false,
              error: error instanceof Error ? error.message : String(error),
            })
        },
      )
    }, 150)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [doc, enabled, generation, key, master, query])

  if (!enabled || doc === null) return EMPTY_SEARCH
  return result?.key === key ? result : { ...EMPTY_SEARCH, loading: true }
}
