import type { Change, DocId } from '../../api'
import { pathCache } from '../../canvas/pathCache'
import { isFiltered } from '../../stores/documentStore'
import { invalidateFontInfo } from '../../stores/fontInfoCache'

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

/** The mounted grid, when there is one: its list identity and its pages. */
export interface InvalidationGrid {
  query: GlyphQuery
  pager: InvalidationPager
}

/**
 * Invalidates every cache affected by a document change.
 *
 * This is the single place the font view and the Font Info tab read a `Change`:
 * both subscribe to the kernel's document events and hand them straight here,
 * so what an edit invalidates is decided once rather than per view. Only one
 * tab's content is mounted at a time, hence `grid` is optional — the shared
 * caches still have to be dropped when the Font Info tab, not the grid, is the
 * one on screen. Other subscribers still read `Change` for their own state:
 * `sessionStore` for the document summary, `useGlyphDetail` and
 * `useLayerOutline` for the glyph tab, `glyphPager` for the list shape.
 *
 * Font-wide and master data live in one shared response, so both `font_info`
 * and `masters` drop it: the Font Info tab reloads and the grid's em box picks
 * up the new vertical metrics.
 *
 * For the grid, derived orders and names are discarded whenever list membership
 * or sorting may have changed. Outline paths remain per-glyph except when glyph
 * handles themselves may have been reordered.
 */
export function invalidateFor(
  change: Change,
  doc: DocId,
  generation: number,
  grid?: InvalidationGrid,
): void {
  if (change.font_info || change.masters) invalidateFontInfo(doc)
  if (!grid) return

  const { query, pager } = grid
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
