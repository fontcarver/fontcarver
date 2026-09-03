import { describe, expect, it, vi } from 'vitest'

import { PathCache } from './pathCache'

/** jsdom has no `Path2D`, so the cache is built with a stand-in factory. */
function fakeFactory(data: string) {
  return { data } as unknown as Path2D
}

describe('PathCache', () => {
  it('builds an outline once per glyph generation', () => {
    const factory = vi.fn(fakeFactory)
    const cache = new PathCache(10, factory)

    const first = cache.get(1, 7, 0, 'M0 0L10 0Z')
    const again = cache.get(1, 7, 0, 'M0 0L10 0Z')

    expect(again).toBe(first)
    expect(factory).toHaveBeenCalledTimes(1)
    expect(cache.hits).toBe(1)
  })

  it('treats a new glyph generation as a different entry', () => {
    const factory = vi.fn(fakeFactory)
    const cache = new PathCache(10, factory)

    cache.get(1, 7, 0, 'M0 0Z')
    cache.get(1, 7, 1, 'M0 0L1 1Z')

    expect(factory).toHaveBeenCalledTimes(2)
    expect(cache.has(1, 7, 0)).toBe(true)
    expect(cache.has(1, 7, 1)).toBe(true)
  })

  it('caches the absence of an outline for empty glyphs', () => {
    const factory = vi.fn(fakeFactory)
    const cache = new PathCache(10, factory)

    expect(cache.get(1, 3, 0, '')).toBeNull()
    expect(cache.get(1, 3, 0, '')).toBeNull()
    expect(factory).not.toHaveBeenCalled()
    expect(cache.size).toBe(1)
  })

  it('evicts the least recently used entry past the limit', () => {
    const cache = new PathCache(2, fakeFactory)

    cache.get(1, 1, 0, 'a')
    cache.get(1, 2, 0, 'b')
    cache.get(1, 1, 0, 'a') // 1 becomes the most recently used entry
    cache.get(1, 3, 0, 'c')

    expect(cache.size).toBe(2)
    expect(cache.has(1, 1, 0)).toBe(true)
    expect(cache.has(1, 3, 0)).toBe(true)
    expect(cache.has(1, 2, 0)).toBe(false)
  })

  it('keeps the same glyph handle of two documents apart', () => {
    const factory = vi.fn(fakeFactory)
    const cache = new PathCache(10, factory)

    const first = cache.get(1, 36, 0, 'M0 0L10 0Z')
    const second = cache.get(2, 36, 0, 'M0 0L20 0Z')

    expect(second).not.toBe(first)
    expect(factory).toHaveBeenCalledTimes(2)

    cache.clearDoc(1)
    expect(cache.has(1, 36, 0)).toBe(false)
    expect(cache.has(2, 36, 0)).toBe(true)
  })

  it('drops every generation of the invalidated glyphs in one pass', () => {
    const cache = new PathCache(10, fakeFactory)
    cache.get(1, 1, 0, 'a')
    cache.get(1, 1, 1, 'a2')
    cache.get(1, 2, 0, 'b')

    cache.drop(1, [1])

    expect(cache.has(1, 1, 0)).toBe(false)
    expect(cache.has(1, 1, 1)).toBe(false)
    expect(cache.has(1, 2, 0)).toBe(true)
  })
})
