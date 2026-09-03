/**
 * Position of a glyph inside the current filtered, sorted glyph list.
 *
 * The glyph tab's "previous / next glyph" commands walk the same order the
 * font view shows, but the kernel indexes glyphs by position and the UI holds
 * handles, so the mapping has to be learned. Every page the grid loads records
 * its positions here, which makes the common case — stepping away from a glyph
 * that was opened from the grid — a single one-cell request. A glyph whose
 * position was never seen falls back to a paged scan, cached the same way.
 */

import { call, type DocId, type GlyphCell, type GlyphId } from '../../api'
import { onDocumentForgotten } from '../../stores/documentStore'

import { MAX_LIST_LIMIT, type GlyphQuery } from './glyphSelection'

/** How many query orders are remembered before the oldest is dropped. */
const MAX_ORDERS = 8

interface OrderEntry {
  doc: DocId
  positions: Map<GlyphId, number>
  total: number | null
}

const orders = new Map<string, OrderEntry>()

/** Stable identity of one filtered, sorted list. */
export function orderKey(query: GlyphQuery): string {
  return JSON.stringify([
    query.doc,
    query.master?.toString() ?? '',
    query.sort,
    query.filter,
  ])
}

function entryFor(query: GlyphQuery): OrderEntry {
  const key = orderKey(query)
  const existing = orders.get(key)
  if (existing) {
    // Re-insert so the most recently used order survives eviction.
    orders.delete(key)
    orders.set(key, existing)
    return existing
  }
  const entry: OrderEntry = {
    doc: query.doc,
    positions: new Map(),
    total: null,
  }
  orders.set(key, entry)
  while (orders.size > MAX_ORDERS) {
    const oldest = orders.keys().next()
    if (oldest.done) break
    orders.delete(oldest.value)
  }
  return entry
}

/** Records the positions of a freshly loaded page. */
export function rememberOrder(
  query: GlyphQuery,
  offset: number,
  cells: readonly GlyphCell[],
  total?: number,
): void {
  const entry = entryFor(query)
  if (total !== undefined) entry.total = total
  cells.forEach((cell, index) => entry.positions.set(cell.gid, offset + index))
}

/** The known position of one glyph, or `undefined` when it was never loaded. */
export function knownIndex(
  query: GlyphQuery,
  gid: GlyphId,
): number | undefined {
  return orders.get(orderKey(query))?.positions.get(gid)
}

/** Forgets every order of one document; called when it closes or reorders. */
export function forgetOrder(doc: DocId): void {
  for (const [key, entry] of [...orders])
    if (entry.doc === doc) orders.delete(key)
}

onDocumentForgotten(forgetOrder)

/** Empties the cache. Used by tests. */
export function resetOrders(): void {
  orders.clear()
}

async function fetchPage(
  query: GlyphQuery,
  offset: number,
  limit: number,
): Promise<GlyphCell[]> {
  const result = await call('glyphs.list', {
    doc: query.doc,
    master: query.master,
    filter: query.filter,
    sort: query.sort,
    offset,
    limit,
    include_paths: false,
  })
  rememberOrder(query, offset, result.items, result.total)
  return result.items
}

/** Finds a glyph's position, scanning the list in pages when it is unknown. */
export async function indexOf(
  query: GlyphQuery,
  gid: GlyphId,
): Promise<number | undefined> {
  const cached = knownIndex(query, gid)
  if (cached !== undefined) return cached

  let offset = 0
  for (;;) {
    const items = await fetchPage(query, offset, MAX_LIST_LIMIT)
    const found = items.findIndex((cell) => cell.gid === gid)
    if (found >= 0) return offset + found
    if (items.length < MAX_LIST_LIMIT) return undefined
    offset += items.length
    const total = orders.get(orderKey(query))?.total
    if (total !== null && total !== undefined && offset >= total)
      return undefined
  }
}

/**
 * The glyph before or after `gid` in the current order, or `null` at either
 * end of the list.
 */
export async function neighbourGid(
  query: GlyphQuery,
  gid: GlyphId,
  direction: 1 | -1,
): Promise<GlyphId | null> {
  const index = await indexOf(query, gid)
  if (index === undefined) return null
  const target = index + direction
  if (target < 0) return null
  const total = orders.get(orderKey(query))?.total
  if (total !== null && total !== undefined && target >= total) return null
  const items = await fetchPage(query, target, 1)
  return items[0]?.gid ?? null
}
