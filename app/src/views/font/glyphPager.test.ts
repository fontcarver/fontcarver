import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { GlyphCell, GlyphListResult } from '../../api'

import { GlyphPager } from './glyphPager'

function makeCell(gid: number): GlyphCell {
  return {
    gid,
    name: `g${gid}`,
    unicodes: [0x41 + gid],
    width: 600,
    bbox: [0, 0, 500, 700],
    path: 'M0 0L1 1Z',
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 0,
  }
}

function page(offset: number, limit: number, total = 65_535): GlyphListResult {
  const count = Math.max(0, Math.min(limit, total - offset))
  return {
    total,
    items: Array.from({ length: count }, (_, index) =>
      makeCell(offset + index),
    ),
  }
}

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('GlyphPager', () => {
  it('requests only the pages that overlap the rendered range', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    pager.setRange(280, 320)
    await vi.advanceTimersByTimeAsync(20)

    expect(fetchPage.mock.calls).toEqual([
      [0, 300],
      [300, 300],
    ])
    expect(pager.total).toBe(65_535)
    expect(pager.cell(310)?.gid).toBe(310)
    expect(pager.cell(900)).toBeUndefined()
  })

  it('debounces a scroll that crosses many rows into one burst', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    pager.setRange(0, 100)
    pager.setRange(1_000, 1_100)
    pager.setRange(20_000, 20_100)
    await vi.advanceTimersByTimeAsync(20)

    expect(fetchPage.mock.calls).toEqual([
      [19_800, 300],
      [20_100, 300],
    ])
  })

  it('never asks twice for a page it already holds', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    pager.setRange(0, 100)
    await vi.advanceTimersByTimeAsync(20)
    pager.setRange(10, 120)
    await vi.advanceTimersByTimeAsync(20)

    expect(fetchPage).toHaveBeenCalledTimes(1)
  })

  it('drops responses issued before a filter change', async () => {
    let release: ((result: GlyphListResult) => void) | undefined
    let issued = 0
    const fetchPage = vi.fn(
      (offset: number, limit: number) =>
        new Promise<GlyphListResult>((resolve) => {
          issued += 1
          if (issued === 1) release = resolve
          else resolve(page(offset, limit, 12))
        }),
    )
    const onChange = vi.fn()
    const pager = new GlyphPager({ fetchPage, onChange })

    pager.setRange(0, 100)
    await vi.advanceTimersByTimeAsync(20)

    pager.reset() // the filter changed while page 0 was in flight
    pager.setRange(0, 100)
    await vi.advanceTimersByTimeAsync(20)
    release?.(page(0, 300, 65_535))
    await vi.advanceTimersByTimeAsync(0)

    expect(pager.total).toBe(12)
    expect(pager.cell(0)?.gid).toBe(0)
    expect(fetchPage).toHaveBeenCalledTimes(2)
  })

  it('refetches the page of a glyph that changed', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    pager.setRange(0, 100)
    await vi.advanceTimersByTimeAsync(20)
    pager.invalidate([5])
    await vi.advanceTimersByTimeAsync(20)

    expect(fetchPage).toHaveBeenCalledTimes(2)
    expect(pager.cell(5)?.gid).toBe(5)
  })

  it('reports a failed page once and lets it be retried', async () => {
    const onError = vi.fn()
    const fetchPage = vi
      .fn<(offset: number, limit: number) => Promise<GlyphListResult>>()
      .mockRejectedValueOnce(new Error('kernel is busy'))
      .mockImplementation(async (offset, limit) => page(offset, limit))
    const pager = new GlyphPager({
      fetchPage,
      onChange: () => undefined,
      onError,
    })

    pager.setRange(0, 10)
    await vi.advanceTimersByTimeAsync(20)
    expect(onError).toHaveBeenCalledTimes(1)

    pager.setRange(0, 11)
    await vi.advanceTimersByTimeAsync(20)
    expect(pager.cell(0)?.gid).toBe(0)
  })

  it('stops requesting once cancelled', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    pager.setRange(0, 100)
    pager.cancel()
    await vi.advanceTimersByTimeAsync(20)

    expect(fetchPage).not.toHaveBeenCalled()
  })
})
