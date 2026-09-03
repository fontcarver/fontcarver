import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { GlyphCell, GlyphListResult } from '../../api'

import { GlyphPager, MAX_CACHED_PAGES, requiresFullReload } from './glyphPager'

function makeCell(gid: number): GlyphCell {
  return {
    gid,
    name: `g${gid}`,
    unicodes: [0x41 + gid],
    category: 'Letter',
    sub_category: 'Uppercase',
    script: 'Latn',
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

describe('GlyphPager invalidation', () => {
  it('drops the whole page a changed glyph sat on and refetches it', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    pager.setRange(0, 100)
    await vi.advanceTimersByTimeAsync(20)
    expect(pager.cell(299)?.gid).toBe(299)

    pager.invalidate([5])

    // Nothing from that page may stay on screen: it is all pre-edit.
    expect(pager.cell(5)).toBeUndefined()
    expect(pager.cell(299)).toBeUndefined()

    await vi.advanceTimersByTimeAsync(20)
    expect(fetchPage).toHaveBeenCalledTimes(2)
    expect(pager.cell(5)?.gid).toBe(5)
  })

  it('discards a page computed before the last document change', async () => {
    let release: ((result: GlyphListResult) => void) | undefined
    let issued = 0
    const fetchPage = vi.fn(
      (offset: number, limit: number) =>
        new Promise<GlyphListResult>((resolve) => {
          issued += 1
          if (issued === 1) release = resolve
          else resolve(page(offset, limit))
        }),
    )
    const onChange = vi.fn()
    const pager = new GlyphPager({ fetchPage, onChange })

    pager.setRange(0, 10)
    await vi.advanceTimersByTimeAsync(20)

    // An edit lands while page 0 is in flight.
    pager.setGeneration(7)
    pager.setRange(0, 10)
    await vi.advanceTimersByTimeAsync(20)

    release?.({ total: 999, items: [makeCell(4242)] })
    await vi.advanceTimersByTimeAsync(0)

    expect(pager.generation).toBe(7)
    expect(pager.cell(0)?.gid).toBe(0)
    expect(pager.total).toBe(65_535)
    expect(fetchPage).toHaveBeenCalledTimes(2)
  })

  it('ignores a generation that does not move forward', () => {
    const pager = new GlyphPager({
      fetchPage: async () => page(0, 300),
      onChange: () => undefined,
    })

    pager.setGeneration(5)
    pager.setGeneration(3)

    expect(pager.generation).toBe(5)
  })
})

describe('GlyphPager memory', () => {
  it('keeps the page cache bounded while traversing 65,535 glyphs', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({ fetchPage, onChange: () => undefined })

    for (let first = 0; first < 65_535; first += 200) {
      pager.setRange(first, Math.min(65_534, first + 199))
      await vi.advanceTimersByTimeAsync(20)
    }

    expect(pager.pageCount).toBeLessThanOrEqual(MAX_CACHED_PAGES)
    expect(pager.cellCount).toBeLessThanOrEqual(MAX_CACHED_PAGES * 300)
    // The tail of the font, which is what is on screen, survived.
    expect(pager.cell(65_534)?.gid).toBe(65_534)
    expect(pager.cell(0)).toBeUndefined()
  })

  it('never evicts a page inside the rendered range', async () => {
    const fetchPage = vi.fn(async (offset: number, limit: number) =>
      page(offset, limit),
    )
    const pager = new GlyphPager({
      fetchPage,
      onChange: () => undefined,
      maxPages: 2,
    })

    pager.setRange(0, 899)
    await vi.advanceTimersByTimeAsync(20)

    expect(pager.pageCount).toBe(3)
    expect(pager.cell(0)?.gid).toBe(0)
    expect(pager.cell(899)?.gid).toBe(899)
  })
})

describe('requiresFullReload', () => {
  const change = (glyphs: number[], order = false) => ({
    glyphs,
    glyph_order: order,
  })

  it('reloads when the glyph order changed', () => {
    expect(requiresFullReload(change([], true), 'Order', false)).toBe(true)
  })

  it('reloads when the sort is derived from a glyph attribute', () => {
    expect(requiresFullReload(change([3]), 'Name', false)).toBe(true)
    expect(requiresFullReload(change([3]), 'Unicode', false)).toBe(true)
    expect(requiresFullReload(change([3]), 'Width', false)).toBe(true)
  })

  it('reloads when a filter decides membership', () => {
    expect(requiresFullReload(change([3]), 'Order', true)).toBe(true)
  })

  it('only invalidates the changed cells in an unfiltered font order', () => {
    expect(requiresFullReload(change([3]), 'Order', false)).toBe(false)
  })

  it('does nothing when no glyph changed', () => {
    expect(requiresFullReload(change([]), 'Name', true)).toBe(false)
  })
})
