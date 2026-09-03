import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { Change, GlyphSort } from '../../api'
import { pathCache } from '../../canvas/pathCache'
import { emptyFilter } from '../../stores/documentStore'
import { cellFor } from '../../test/glyphFixtures'

import { forgetGlyphNames, glyphName, rememberGlyphNames } from './glyphNames'
import {
  forgetOrder,
  knownIndex,
  rememberOrder,
  resetOrders,
} from './glyphOrder'
import { invalidateFor, type InvalidationPager } from './invalidation'
import type { GlyphQuery } from './glyphSelection'

const DOC = 41
const CHANGED_GID = 7
const UNCHANGED_GID = 8

function query(sort: GlyphSort = 'Order'): GlyphQuery {
  return { doc: DOC, master: null, filter: emptyFilter(), sort }
}

function change(overrides: Partial<Change> = {}): Change {
  return {
    glyphs: [CHANGED_GID],
    font_info: false,
    masters: false,
    glyph_order: false,
    ...overrides,
  }
}

function fakePager(): InvalidationPager {
  return {
    setGeneration: vi.fn(),
    reset: vi.fn(),
    setRange: vi.fn(),
    invalidate: vi.fn(),
  }
}

function seedCaches(activeQuery: GlyphQuery): void {
  const cells = [cellFor(CHANGED_GID), cellFor(UNCHANGED_GID)]
  rememberOrder(activeQuery, 3, cells, 10)
  rememberGlyphNames(DOC, cells)
  pathCache.get(DOC, CHANGED_GID, 1, null)
  pathCache.get(DOC, UNCHANGED_GID, 1, null)
}

beforeEach(() => {
  resetOrders()
  forgetGlyphNames(DOC)
  forgetOrder(DOC)
  pathCache.clear()
})

describe('invalidateFor', () => {
  it('fully invalidates order, names and paths after a glyph-order change', () => {
    const activeQuery = query()
    const pager = fakePager()
    seedCaches(activeQuery)

    invalidateFor(
      change({ glyphs: [], glyph_order: true }),
      activeQuery,
      4,
      pager,
    )

    expect(pager.setGeneration).toHaveBeenCalledWith(4)
    expect(pager.reset).toHaveBeenCalledOnce()
    expect(pager.setRange).toHaveBeenCalledWith(0, 0)
    expect(knownIndex(activeQuery, CHANGED_GID)).toBeUndefined()
    expect(glyphName(DOC, CHANGED_GID)).toBeUndefined()
    expect(pathCache.size).toBe(0)
  })

  it('fully reloads an attribute change under Unicode sort', () => {
    const activeQuery = query('Unicode')
    const pager = fakePager()
    seedCaches(activeQuery)

    invalidateFor(change(), activeQuery, 5, pager)

    expect(pager.reset).toHaveBeenCalledOnce()
    expect(pager.invalidate).not.toHaveBeenCalled()
    expect(knownIndex(activeQuery, CHANGED_GID)).toBeUndefined()
    expect(glyphName(DOC, CHANGED_GID)).toBeUndefined()
  })

  it('fully reloads an attribute change with an active filter', () => {
    const activeQuery = query()
    activeQuery.filter.query = 'A'
    const pager = fakePager()
    seedCaches(activeQuery)

    invalidateFor(change(), activeQuery, 6, pager)

    expect(pager.reset).toHaveBeenCalledOnce()
    expect(knownIndex(activeQuery, CHANGED_GID)).toBeUndefined()
    expect(glyphName(DOC, CHANGED_GID)).toBeUndefined()
  })

  it('keeps order and names for a plain outline change under Order sort', () => {
    const activeQuery = query()
    const pager = fakePager()
    seedCaches(activeQuery)

    invalidateFor(change(), activeQuery, 7, pager)

    expect(pager.reset).not.toHaveBeenCalled()
    expect(pager.invalidate).toHaveBeenCalledWith([CHANGED_GID])
    expect(knownIndex(activeQuery, CHANGED_GID)).toBe(3)
    expect(glyphName(DOC, CHANGED_GID)).toBe(cellFor(CHANGED_GID).name)
    expect(pathCache.has(DOC, CHANGED_GID, 1)).toBe(false)
    expect(pathCache.has(DOC, UNCHANGED_GID, 1)).toBe(true)
  })
})
