import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, expect, it } from 'vitest'

import { setTransport, type GlyphCell, type GlyphListResult } from '../api'
import { createMockTransport } from '../test/mockTransport'

import { usePaletteGlyphSearch } from './usePaletteGlyphSearch'

afterEach(() => setTransport(undefined))

const glyph = (name: string): GlyphCell => ({
  gid: 42,
  name,
  unicodes: [65],
  category: 'Letter',
  sub_category: 'Uppercase',
  script: 'Latn',
  width: 600,
  bbox: null,
  path: null,
  color_label: null,
  export: true,
  has_components: false,
  is_empty: true,
  generation: 0,
})

it('loads at most fifty matching glyphs without paths or the active grid filter', async () => {
  const transport = createMockTransport({
    'glyphs.list': () => ({ total: 100, items: [glyph('A')] }),
  })
  setTransport(transport)
  const { result } = renderHook(() =>
    usePaletteGlyphSearch(1, null, 0, 'A', true),
  )
  await waitFor(() => expect(result.current.items[0]?.name).toBe('A'))
  expect(transport.calls).toEqual([
    {
      method: 'glyphs.list',
      params: expect.objectContaining({
        doc: 1,
        filter: expect.objectContaining({
          query: 'A',
          categories: [],
          unencoded_only: false,
        }),
        sort: 'Name',
        offset: 0,
        limit: 50,
        include_paths: false,
      }),
    },
  ])
  expect(result.current.total).toBe(100)
})

it('hides stale matches immediately and ignores a slower old response', async () => {
  let old: (value: GlyphListResult) => void = () => undefined
  const transport = createMockTransport({
    'glyphs.list': (params) =>
      params.filter.query === 'A'
        ? new Promise((resolve) => {
            old = resolve
          })
        : { total: 1, items: [glyph('B')] },
  })
  setTransport(transport)
  const { result, rerender } = renderHook(
    ({ query }) => usePaletteGlyphSearch(1, null, 0, query, true),
    { initialProps: { query: 'A' } },
  )
  await waitFor(() => expect(transport.calls).toHaveLength(1))
  rerender({ query: 'B' })
  expect(result.current.items).toEqual([])
  await waitFor(() => expect(result.current.items[0]?.name).toBe('B'))
  await act(async () => old({ total: 1, items: [glyph('A')] }))
  expect(result.current.items[0]?.name).toBe('B')
})

it('does not request glyphs when search has no current document', () => {
  const transport = createMockTransport({})
  setTransport(transport)
  const { result } = renderHook(() =>
    usePaletteGlyphSearch(null, null, 0, 'A', true),
  )
  expect(result.current.items).toEqual([])
  expect(result.current.loading).toBe(false)
  expect(transport.calls).toEqual([])
})
