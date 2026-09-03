/**
 * Selection helpers for the glyph grid.
 *
 * Selection is stored as `GlyphId`s, but the grid navigates by position in the
 * current filtered, sorted order. Ranges and "select all" therefore have to
 * translate positions into handles, which needs the kernel whenever the cells
 * are not in the page cache. Outlines are never requested for these calls.
 */

import {
  call,
  type DocId,
  type GlyphFilter,
  type GlyphId,
  type GlyphSort,
  type MasterId,
} from '../../api'

import { rememberGlyphNames } from './glyphNames'

/** Largest page the kernel accepts for `glyphs.list`. */
export const MAX_LIST_LIMIT = 2_000

/** The query one glyph list is taken from. */
export interface GlyphQuery {
  doc: DocId
  master: MasterId | null
  filter: GlyphFilter
  sort: GlyphSort
}

/**
 * Fetches the glyph handles at `[offset, offset + count)` of a query, paging
 * through the kernel's maximum page size. Names are recorded on the way so a
 * later "Copy Glyph Names" needs no further request.
 */
export async function fetchGidRange(
  query: GlyphQuery,
  offset: number,
  count: number,
): Promise<GlyphId[]> {
  const gids: GlyphId[] = []
  let position = Math.max(0, offset)
  let remaining = Math.max(0, count)
  while (remaining > 0) {
    const limit = Math.min(MAX_LIST_LIMIT, remaining)
    const result = await call('glyphs.list', {
      doc: query.doc,
      master: query.master,
      filter: query.filter,
      sort: query.sort,
      offset: position,
      limit,
      include_paths: false,
    })
    rememberGlyphNames(query.doc, result.items)
    for (const cell of result.items) gids.push(cell.gid)
    if (result.items.length < limit) break
    position += result.items.length
    remaining -= result.items.length
    if (position >= result.total) break
  }
  return gids
}

/** The inclusive index range between two positions, in ascending order. */
export function rangeBetween(anchor: number, target: number): [number, number] {
  return anchor <= target ? [anchor, target] : [target, anchor]
}

/** Adds `gids` to `selection`, returning a new set. */
export function withAdded(
  selection: ReadonlySet<GlyphId>,
  gids: Iterable<GlyphId>,
): Set<GlyphId> {
  const next = new Set(selection)
  for (const gid of gids) next.add(gid)
  return next
}

/** Removes `gid` when present, adds it otherwise. */
export function withToggled(
  selection: ReadonlySet<GlyphId>,
  gid: GlyphId,
): Set<GlyphId> {
  const next = new Set(selection)
  if (next.has(gid)) next.delete(gid)
  else next.add(gid)
  return next
}
