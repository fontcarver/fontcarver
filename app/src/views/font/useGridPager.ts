/**
 * The glyph grid's data side: one {@link GlyphPager} and everything that keeps
 * it in step with the kernel.
 *
 * Separated from the component so the grid itself is left with layout and
 * input: page requests, the match count the status bar shows, invalidation on
 * document events and the development probe all live here.
 */

import { useEffect, useReducer, useState } from 'react'

import { call, onDocumentEvent, type DocId } from '../../api'
import { useDocumentStore } from '../../stores/documentStore'
import { reportError } from '../../stores/uiStore'

import { rememberGlyphNames } from './glyphNames'
import { rememberOrder } from './glyphOrder'
import { GlyphPager } from './glyphPager'
import type { GlyphQuery } from './glyphSelection'
import { invalidateFor } from './invalidation'

/** What {@link useGridPager} needs to fetch and invalidate pages. */
export interface GridPagerOptions {
  doc: DocId
  /** The filtered, sorted list; stable for the lifetime of one grid mount. */
  query: GlyphQuery
}

/** The pager and the render trigger that follows its cached cells. */
export interface GridPager {
  pager: GlyphPager
  /** Bumped whenever cached cells or the total changed. */
  version: number
  /** Cells in the current filter; `0` until the first page answers. */
  total: number
}

/** Creates the pager for one glyph list and keeps it loaded and fresh. */
export function useGridPager({ doc, query }: GridPagerOptions): GridPager {
  const setMatchCount = useDocumentStore((state) => state.setMatchCount)
  const [version, bump] = useReducer((count: number) => count + 1, 0)

  const [pager] = useState(
    () =>
      new GlyphPager({
        fetchPage: async (offset, limit) => {
          const result = await call('glyphs.list', {
            doc: query.doc,
            master: query.master,
            filter: query.filter,
            sort: query.sort,
            offset,
            limit,
            include_paths: true,
          })
          rememberGlyphNames(query.doc, result.items)
          rememberOrder(query, offset, result.items, result.total)
          return result
        },
        onChange: bump,
        onError: reportError,
      }),
  )

  useEffect(() => {
    if (!import.meta.env.DEV) return
    // Development probe: lets a browser agent confirm that the page cache
    // stays bounded while scrolling a 65k-glyph font.
    const target = globalThis as { __fwGridStats?: () => unknown }
    target.__fwGridStats = () => ({
      pages: pager.pageCount,
      cells: pager.cellCount,
      total: pager.total,
    })
    return () => {
      delete target.__fwGridStats
    }
  }, [pager])

  useEffect(() => () => pager.cancel(), [pager])

  useEffect(() => {
    if (pager.total !== null) setMatchCount(doc, pager.total)
  }, [doc, pager, setMatchCount, version])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc) return
        invalidateFor(event.change, doc, event.generation, { query, pager })
      }),
    [doc, pager, query],
  )

  return { pager, version, total: pager.total ?? 0 }
}
