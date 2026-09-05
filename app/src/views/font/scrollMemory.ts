/**
 * Where the glyph grid was scrolled to, per filtered and sorted list.
 *
 * An edit that changes list membership or glyph order forces a full reload:
 * the pager drops every page, the content collapses to nothing and the browser
 * clamps the scroll offset to zero. Without a memory, a rename in a 65k-glyph
 * font would throw the user back to the top of the font. The offset is kept
 * here rather than in the document store because it is written while scrolling,
 * and a store write per row would re-render every view that watches the
 * document. It is keyed by the list identity, so a new filter or sort starts at
 * the top — the same rule the grid's page cache follows.
 */

import type { DocId } from '../../api'

import { orderKey } from './glyphOrder'
import type { GlyphQuery } from './glyphSelection'

/** How many lists keep an offset before the least recently used one is dropped. */
const MAX_REMEMBERED = 8

const offsets = new Map<string, { doc: DocId; top: number }>()

/** Records where one list is scrolled to, in CSS pixels. */
export function rememberScroll(query: GlyphQuery, top: number): void {
  const key = orderKey(query)
  // Re-insert so the most recently used list survives eviction.
  offsets.delete(key)
  offsets.set(key, { doc: query.doc, top })
  while (offsets.size > MAX_REMEMBERED) {
    const oldest = offsets.keys().next()
    if (oldest.done) break
    offsets.delete(oldest.value)
  }
}

/** The offset last recorded for one list, or `null` when it has none. */
export function rememberedScroll(query: GlyphQuery): number | null {
  return offsets.get(orderKey(query))?.top ?? null
}

/** Forgets every list of one document; called when the document closes. */
export function forgetScroll(doc: DocId): void {
  for (const [key, entry] of [...offsets])
    if (entry.doc === doc) offsets.delete(key)
}

/** Empties the memory. Used by tests. */
export function resetScrollMemory(): void {
  offsets.clear()
}
