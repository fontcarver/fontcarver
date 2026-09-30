import { describe, expect, it, vi } from 'vitest'
import { ColorGlyphCache } from './colorGlyphCache'

const resource = () => ({ paint: vi.fn(), dispose: vi.fn() })

describe('bounded color glyph cache', () => {
  it('deduplicates leases and releases loaded resources on disposal', async () => {
    const paint = resource()
    const load = vi.fn(async () => paint)
    const cache = new ColorGlyphCache(load, 2, 1)
    const first = cache.acquire('A')
    const second = cache.acquire('A')
    expect(await first.result).toBe(paint)
    expect(await second.result).toBe(paint)
    expect(load).toHaveBeenCalledTimes(1)
    first.release()
    second.release()
    cache.dispose()
    expect(paint.dispose).toHaveBeenCalledTimes(1)
  })

  it('bounds requests, evicts released entries and drops late disposed results', async () => {
    const paint = resource()
    let resolve!: (value: typeof paint) => void
    const load = vi.fn(
      () =>
        new Promise<typeof paint>((done) => {
          resolve = done
        }),
    )
    const cache = new ColorGlyphCache(load, 1, 1)
    const first = cache.acquire('A')
    expect(await cache.acquire('B').result).toBeNull()
    expect(load).toHaveBeenCalledTimes(1)
    first.release()
    cache.dispose()
    resolve(paint)
    expect(await first.result).toBeNull()
    expect(paint.dispose).toHaveBeenCalledTimes(1)
  })

  it('limits concurrent loading and releases queued requests without running them', async () => {
    const resolvers: (() => void)[] = []
    const load = vi.fn<(glyph: string) => Promise<null>>(
      () =>
        new Promise<null>((resolve) => {
          resolvers.push(() => resolve(null))
        }),
    )
    const cache = new ColorGlyphCache(load, 10, 2)
    const leases = ['A', 'B', 'C', 'D'].map((name) => cache.acquire(name))
    expect(load).toHaveBeenCalledTimes(2)
    leases[2]!.release()
    resolvers[0]!()
    await leases[0]!.result
    await vi.waitFor(() => expect(load).toHaveBeenCalledTimes(3))
    expect(load.mock.calls.map((args) => args[0])).toEqual(['A', 'B', 'D'])
    cache.dispose()
    resolvers.slice(1).forEach((resolve) => resolve())
    await Promise.all(leases.map((lease) => lease.result))
  })
})
