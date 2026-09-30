/**
 * How many glyphs each saved smart filter matches.
 *
 * A predicate has no repertoire, so a filter's badge is "matches / glyphs in the
 * font". Each count is one `glyphs.list` request with no page, evaluated over
 * every glyph, so they are fetched one at a time and only while the project has
 * few enough saved filters for that to stay cheap.
 */

import { useEffect, useState } from 'react'

import { call, type DocId } from '../../api'
import { emptyFilter } from '../../stores/documentStore'

/** Beyond this many saved filters the counts are not fetched. */
export const MAX_COUNTED_FILTERS = 32

/** Matches per canonical expression; a missing entry has not answered yet. */
export type SmartFilterCounts = ReadonlyMap<string, number>

/** Loads a match count per expression, sequentially and cancellably. */
export function useSmartFilterCounts(
  doc: DocId,
  expressions: readonly string[],
  generation: number,
): SmartFilterCounts {
  const [counts, setCounts] = useState<{
    key: string
    values: ReadonlyMap<string, number>
  }>({ key: '', values: new Map() })
  const key = `${doc}:${generation}:${expressions.join('\u0000')}`

  useEffect(() => {
    // Nothing to count: the render below already falls back to an empty map,
    // because its key no longer matches.
    if (expressions.length === 0 || expressions.length > MAX_COUNTED_FILTERS) {
      return
    }
    let live = true
    void (async () => {
      const values = new Map<string, number>()
      for (const expression of expressions) {
        try {
          const page = await call('glyphs.list', {
            doc,
            master: null,
            filter: { ...emptyFilter(), smart_expression: expression },
            sort: 'Order',
            offset: 0,
            limit: 0,
            include_paths: false,
          })
          if (!live) return
          values.set(expression, page.total)
        } catch {
          // A filter the kernel rejects simply shows no count.
          if (!live) return
        }
        if (live) setCounts({ key, values: new Map(values) })
      }
    })()
    return () => {
      live = false
    }
    // The key covers the document, its generation and the expression list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key])

  return counts.key === key ? counts.values : new Map()
}
