import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type GlyphListParams } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { makeCell } from '../../test/fontFixtures'
import { emptyFilter } from '../../stores/documentStore'

import {
  indexOf,
  knownIndex,
  neighbourGid,
  rememberOrder,
  resetOrders,
} from './glyphOrder'
import type { GlyphQuery } from './glyphSelection'

const query: GlyphQuery = {
  doc: 1,
  master: 'm1',
  filter: emptyFilter(),
  sort: 'Order',
}

/** A list whose gid at position `n` is `n + 100`. */
function transportFor(total: number) {
  return createMockTransport({
    'glyphs.index_of': ({ gid }) => {
      const index = gid - 100
      return index >= 0 && index < total ? index : null
    },
    'glyphs.list': (params: GlyphListParams) => ({
      total,
      items: Array.from(
        { length: Math.max(0, Math.min(params.limit, total - params.offset)) },
        (_, index) =>
          makeCell({
            gid: params.offset + index + 100,
            name: `g${params.offset + index}`,
          }),
      ),
    }),
  })
}

beforeEach(() => resetOrders())
afterEach(() => setTransport(undefined))

describe('glyph order', () => {
  it('records the positions of a loaded page', () => {
    rememberOrder(query, 40, [makeCell({ gid: 7 }), makeCell({ gid: 8 })], 500)
    expect(knownIndex(query, 7)).toBe(40)
    expect(knownIndex(query, 8)).toBe(41)
    expect(knownIndex(query, 9)).toBeUndefined()
  })

  it('keeps orders of different queries apart', () => {
    rememberOrder(query, 0, [makeCell({ gid: 7 })])
    expect(knownIndex({ ...query, sort: 'Unicode' }, 7)).toBeUndefined()
  })

  it('steps to the next glyph with one request when the index is known', async () => {
    const transport = transportFor(500)
    setTransport(transport)
    rememberOrder(query, 10, [makeCell({ gid: 110 })], 500)

    await expect(neighbourGid(query, 110, 1)).resolves.toBe(111)

    const lists = transport.calls.filter(
      (entry) => entry.method === 'glyphs.list',
    )
    expect(lists).toHaveLength(1)
    expect(lists[0]?.params).toMatchObject({
      offset: 11,
      limit: 1,
      include_paths: false,
    })
  })

  it('steps backwards and stops at the start of the list', async () => {
    setTransport(transportFor(500))
    rememberOrder(query, 1, [makeCell({ gid: 101 })], 500)
    await expect(neighbourGid(query, 101, -1)).resolves.toBe(100)

    rememberOrder(query, 0, [makeCell({ gid: 100 })], 500)
    await expect(neighbourGid(query, 100, -1)).resolves.toBeNull()
  })

  it('stops at the end of the list', async () => {
    setTransport(transportFor(500))
    rememberOrder(query, 499, [makeCell({ gid: 599 })], 500)
    await expect(neighbourGid(query, 599, 1)).resolves.toBeNull()
  })

  it('asks for one index instead of scanning pages for an unseen glyph', async () => {
    const transport = createMockTransport({
      'glyphs.index_of': () => 2_100,
      'glyphs.list': (params: GlyphListParams) => ({
        total: 2_500,
        items:
          params.offset === 2_101
            ? [makeCell({ gid: 2_201, name: 'g2101' })]
            : [],
      }),
    })
    setTransport(transport)

    await expect(neighbourGid(query, 2_200, 1)).resolves.toBe(2_201)

    const indexes = transport.calls.filter(
      (entry) => entry.method === 'glyphs.index_of',
    )
    const lists = transport.calls.filter(
      (entry) => entry.method === 'glyphs.list',
    )
    expect(indexes).toHaveLength(1)
    expect(indexes[0]?.params).toEqual({ ...query, gid: 2_200 })
    expect(lists).toHaveLength(1)
    expect(lists[0]?.params).toMatchObject({ offset: 2_101, limit: 1 })
    // The lookup also fills the cache, so the next lookup costs nothing.
    expect(knownIndex(query, 2_200)).toBe(2_100)
  })

  it('reports an unknown glyph as absent', async () => {
    setTransport(transportFor(10))
    await expect(indexOf(query, 9_999)).resolves.toBeUndefined()
    await expect(neighbourGid(query, 9_999, 1)).resolves.toBeNull()
  })
})
