import { useCallback, useEffect, useMemo, useState } from 'react'
import { call, type DocId } from '../../api'
import { emptyFilter } from '../../stores/documentStore'

const emptyNames: readonly string[] = []

/** Completion inventory is bounded independently of the editor source size. */
export function useFeatureCompletions(doc: DocId, generation: number) {
  // A new generation must also recreate the editor's dynamic completion source.
  const request = useMemo(() => ({ doc, generation }), [doc, generation])
  const [inventory, setInventory] = useState<{
    request: typeof request
    names: readonly string[]
  } | null>(null)
  useEffect(() => {
    let stale = false
    void call('glyphs.list', {
      doc: request.doc,
      master: null,
      filter: emptyFilter(),
      sort: 'Name',
      offset: 0,
      limit: 2000,
      include_paths: false,
    })
      .then((result) => {
        if (!stale) {
          setInventory({
            request,
            names: result.items.map((item) => item.name),
          })
        }
      })
      .catch(() => {
        if (!stale) setInventory({ request, names: emptyNames })
      })
    return () => {
      stale = true
    }
  }, [request])
  const loadGlyphNames = useCallback(
    async (query: string) => {
      const result = await call('glyphs.list', {
        doc: request.doc,
        master: null,
        filter: { ...emptyFilter(), query: query || null },
        sort: 'Name',
        offset: 0,
        limit: 200,
        include_paths: false,
      })
      return result.items.map((item) => item.name)
    },
    [request],
  )
  const names = inventory?.request === request ? inventory.names : emptyNames
  return { names, loadGlyphNames }
}
