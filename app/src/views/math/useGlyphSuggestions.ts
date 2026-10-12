import { createContext, useEffect, useState } from 'react'
import { call, type DocId } from '../../api'
import { emptyFilter } from '../../stores/documentStore'

const LIMIT = 50

/** A bounded list of font glyph names matching `query`, for datalist hints. */
export function useGlyphSuggestions(doc: DocId, query: string): string[] {
  const [names, setNames] = useState<string[]>([])
  useEffect(() => {
    let stale = false
    void call('glyphs.list', {
      doc,
      master: null,
      filter: { ...emptyFilter(), query: query.trim() || null },
      sort: 'Name',
      offset: 0,
      limit: LIMIT,
      include_paths: false,
    })
      .then((result) => {
        if (!stale) setNames(result.items.map((item) => item.name))
      })
      .catch(() => undefined)
    return () => {
      stale = true
    }
  }, [doc, query])
  return names
}

/** Glyph-name inputs report their text so suggestions follow the active field. */
export const GlyphQueryContext = createContext<(query: string) => void>(
  () => undefined,
)
