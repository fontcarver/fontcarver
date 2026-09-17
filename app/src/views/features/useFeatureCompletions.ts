import { useCallback, useEffect, useState } from 'react'
import { call, type DocId } from '../../api'
import { emptyFilter } from '../../stores/documentStore'

/** Completion inventory is bounded independently of the editor source size. */
export function useFeatureCompletions(doc: DocId, generation: number) {
  const [names, setNames] = useState<readonly string[]>([])
  useEffect(() => {
    let stale = false
    void call('glyphs.list', {
      doc,
      master: null,
      filter: emptyFilter(),
      sort: 'Name',
      offset: 0,
      limit: 2000,
      include_paths: false,
    })
      .then((result) => {
        if (!stale) setNames(result.items.map((item) => item.name))
      })
      .catch(() => undefined)
    return () => {
      stale = true
    }
  }, [doc, generation])
  const loadGlyphNames = useCallback(
    async (query: string) => {
      const result = await call('glyphs.list', {
        doc,
        master: null,
        filter: { ...emptyFilter(), query: query || null },
        sort: 'Name',
        offset: 0,
        limit: 200,
        include_paths: false,
      })
      return result.items.map((item) => item.name)
    },
    [doc],
  )
  return { names, loadGlyphNames }
}
