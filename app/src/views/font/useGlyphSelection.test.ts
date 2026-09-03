import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import type { GlyphListParams, GlyphListResult } from '../../api'
import { setTransport } from '../../api'
import {
  emptyFilter,
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { cellFor, glyphPage } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'

import type { GlyphPager } from './glyphPager'
import type { GlyphQuery } from './glyphSelection'
import { useGlyphSelection } from './useGlyphSelection'

const DOC = 1
const TOTAL = 600
const QUERY: GlyphQuery = {
  doc: DOC,
  master: null,
  filter: emptyFilter(),
  sort: 'Order',
}

/** Only the first ten cells are in the page cache; the rest need the kernel. */
const PAGER = {
  cell: (index: number) => (index < 10 ? cellFor(index) : undefined),
} as unknown as GlyphPager

function selection(): number[] {
  return [...(useDocumentStore.getState().byDoc[DOC]?.selection ?? [])].sort(
    (left, right) => left - right,
  )
}

/**
 * A transport whose range lookups (`include_paths: false`) resolve only when
 * the test says so, so two of them can be interleaved.
 */
function deferredTransport() {
  const pending: ((result: GlyphListResult) => void)[] = []
  const transport = createMockTransport({
    'glyphs.list': (params: GlyphListParams) => {
      if (params.include_paths)
        return glyphPage(params.offset, params.limit, TOTAL)
      return new Promise<GlyphListResult>((resolve) => pending.push(resolve))
    },
  })
  return { transport, pending }
}

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(DOC)
})

afterEach(() => {
  setTransport(undefined)
})

describe('useGlyphSelection', () => {
  it('lets only the newest range request write the selection', async () => {
    const { transport, pending } = deferredTransport()
    setTransport(transport)
    const { result } = renderHook(() =>
      useGlyphSelection({
        doc: DOC,
        query: QUERY,
        pager: PAGER,
        total: TOTAL,
        scrollTo: () => undefined,
      }),
    )

    act(() => result.current.focusAt(500, false))
    act(() => result.current.focusAt(300, false))
    await act(async () => {
      // The newer request answers first, the older one afterwards.
      pending[1]?.({ total: TOTAL, items: [cellFor(300)] })
      await Promise.resolve()
      pending[0]?.({ total: TOTAL, items: [cellFor(500)] })
      await Promise.resolve()
    })

    expect(pending).toHaveLength(2)
    expect(selection()).toEqual([300])
  })

  it('lets a click supersede a range request still in flight', async () => {
    const { transport, pending } = deferredTransport()
    setTransport(transport)
    const { result } = renderHook(() =>
      useGlyphSelection({
        doc: DOC,
        query: QUERY,
        pager: PAGER,
        total: TOTAL,
        scrollTo: () => undefined,
      }),
    )

    act(() => result.current.focusAt(500, false))
    act(() => result.current.selectAt(3, { shift: false, additive: false }))
    expect(selection()).toEqual([3])

    await act(async () => {
      pending[0]?.({ total: TOTAL, items: [cellFor(500)] })
      await Promise.resolve()
    })

    expect(selection()).toEqual([3])
  })

  it('drops a range request that resolves after the grid was replaced', async () => {
    const { transport, pending } = deferredTransport()
    setTransport(transport)
    const { result, unmount } = renderHook(() =>
      useGlyphSelection({
        doc: DOC,
        query: QUERY,
        pager: PAGER,
        total: TOTAL,
        scrollTo: () => undefined,
      }),
    )

    act(() => result.current.focusAt(500, false))
    unmount()
    await act(async () => {
      pending[0]?.({ total: TOTAL, items: [cellFor(500)] })
      await Promise.resolve()
    })

    expect(selection()).toEqual([])
  })

  it('serves a range from the page cache without asking the kernel', async () => {
    const { transport, pending } = deferredTransport()
    setTransport(transport)
    const { result } = renderHook(() =>
      useGlyphSelection({
        doc: DOC,
        query: QUERY,
        pager: PAGER,
        total: TOTAL,
        scrollTo: () => undefined,
      }),
    )

    act(() => result.current.selectAt(1, { shift: false, additive: false }))
    await act(async () => {
      result.current.selectAt(4, { shift: true, additive: false })
      await Promise.resolve()
    })

    expect(pending).toHaveLength(0)
    expect(selection()).toEqual([1, 2, 3, 4])
  })
})
