import type { Change } from '../../api'
import { pathCache } from '../../canvas/pathCache'
import { isFiltered } from '../../stores/documentStore'

import { forgetGlyphNames } from './glyphNames'
import { forgetOrder } from './glyphOrder'
import { requiresFullReload } from './glyphPager'
import type { GlyphQuery } from './glyphSelection'

/** Pager operations used when a document change invalidates grid data. */
export interface InvalidationPager {
  setGeneration: (generation: number) => void
  reset: () => void
  setRange: (first: number, last: number) => void
  invalidate: (gids: Iterable<number>) => void
}

/**
 * Invalidates every grid cache affected by a document change.
 *
 * Derived orders and names are discarded whenever list membership or sorting
 * may have changed. Outline paths remain per-glyph except when glyph handles
 * themselves may have been reordered.
 */
export function invalidateFor(
  change: Change,
  query: GlyphQuery,
  generation: number,
  pager: InvalidationPager,
): void {
  pager.setGeneration(generation)
  const fullReload = requiresFullReload(
    change,
    query.sort,
    isFiltered(query.filter),
  )

  if (fullReload) {
    forgetOrder(query.doc)
    forgetGlyphNames(query.doc)
  }

  if (change.glyph_order) pathCache.clearDoc(query.doc)
  else pathCache.drop(query.doc, change.glyphs)

  if (fullReload) {
    pager.reset()
    pager.setRange(0, 0)
  } else {
    pager.invalidate(change.glyphs)
  }
}
