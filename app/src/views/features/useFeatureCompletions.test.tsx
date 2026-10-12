import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type GlyphListResult } from '../../api'
import { glyphPage } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'

import { useFeatureCompletions } from './useFeatureCompletions'

afterEach(() => setTransport(undefined))

function pendingInventory() {
  const requests: {
    resolve: (result: GlyphListResult) => void
    reject: (error: Error) => void
  }[] = []
  const transport = createMockTransport({
    'glyphs.list': () =>
      new Promise<GlyphListResult>((resolve, reject) => {
        requests.push({ resolve, reject })
      }),
  })
  setTransport(transport)
  return { requests, transport }
}

function inventory(name: string) {
  return glyphPage(0, 1, 1, () => name)
}

describe('useFeatureCompletions', () => {
  it.each([
    ['document', { doc: 2, generation: 1 }],
    ['generation', { doc: 1, generation: 2 }],
  ])(
    'hides the old inventory while the %s refresh is pending',
    async (_, next) => {
      const { requests, transport } = pendingInventory()
      const hook = renderHook(
        ({ doc, generation }) => useFeatureCompletions(doc, generation),
        { initialProps: { doc: 1, generation: 1 } },
      )
      await act(async () => requests[0].resolve(inventory('oldGlyph')))
      expect(hook.result.current.names).toEqual(['oldGlyph'])

      hook.rerender(next)
      expect(hook.result.current.names).toEqual([])
      expect(transport.calls.at(-1)).toMatchObject({
        method: 'glyphs.list',
        params: { doc: next.doc, limit: 2000, include_paths: false },
      })

      await act(async () => requests[1].resolve(inventory('newGlyph')))
      expect(hook.result.current.names).toEqual(['newGlyph'])
    },
  )

  it('keeps failed refreshes empty and recovers on the next generation', async () => {
    const { requests } = pendingInventory()
    const hook = renderHook(
      ({ generation }) => useFeatureCompletions(1, generation),
      { initialProps: { generation: 1 } },
    )
    await act(async () => requests[0].resolve(inventory('removedGlyph')))

    hook.rerender({ generation: 2 })
    await act(async () => requests[1].reject(new Error('offline')))
    expect(hook.result.current.names).toEqual([])

    hook.rerender({ generation: 3 })
    await act(async () => requests[2].resolve(inventory('renamedGlyph')))
    expect(hook.result.current.names).toEqual(['renamedGlyph'])
  })

  it('ignores a superseded inventory that resolves after the latest request', async () => {
    const { requests } = pendingInventory()
    const hook = renderHook(
      ({ generation }) => useFeatureCompletions(1, generation),
      { initialProps: { generation: 1 } },
    )
    hook.rerender({ generation: 2 })
    await act(async () => requests[1].resolve(inventory('currentGlyph')))
    await act(async () => requests[0].resolve(inventory('removedGlyph')))
    expect(hook.result.current.names).toEqual(['currentGlyph'])
  })

  it('refreshes the dynamic loader for each document generation and bounds queries', async () => {
    const transport = createMockTransport({
      'glyphs.list': ({ filter }) => inventory(filter.query ?? 'allGlyphs'),
    })
    setTransport(transport)
    const hook = renderHook(
      ({ doc, generation }) => useFeatureCompletions(doc, generation),
      { initialProps: { doc: 1, generation: 1 } },
    )
    const initialLoader = hook.result.current.loadGlyphNames
    hook.rerender({ doc: 1, generation: 1 })
    expect(hook.result.current.loadGlyphNames).toBe(initialLoader)

    hook.rerender({ doc: 1, generation: 2 })
    const refreshedLoader = hook.result.current.loadGlyphNames
    expect(refreshedLoader).not.toBe(initialLoader)

    hook.rerender({ doc: 2, generation: 2 })
    expect(hook.result.current.loadGlyphNames).not.toBe(refreshedLoader)
    await act(async () => {
      expect(await hook.result.current.loadGlyphNames('a.alt')).toEqual([
        'a.alt',
      ])
    })
    expect(transport.calls.at(-1)).toMatchObject({
      method: 'glyphs.list',
      params: {
        doc: 2,
        filter: { query: 'a.alt' },
        sort: 'Name',
        offset: 0,
        limit: 200,
        include_paths: false,
      },
    })
    await act(async () => {
      expect(await hook.result.current.loadGlyphNames('')).toEqual([
        'allGlyphs',
      ])
    })
    expect(transport.calls.at(-1)).toMatchObject({
      method: 'glyphs.list',
      params: { doc: 2, filter: { query: null }, limit: 200 },
    })
  })
})
