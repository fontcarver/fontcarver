/**
 * Outline cache for the glyph grid.
 *
 * Building a `Path2D` from SVG path data is the single most expensive step of
 * painting a cell, so every outline is kept keyed by document, glyph handle
 * *and* glyph generation: an edited glyph gets a new generation and therefore a
 * new entry, while every other cached outline survives. Glyph handles are only
 * unique inside one document, so the document is part of the key as well. The
 * cache is a plain insertion-ordered LRU, so a 65k-glyph font cannot grow it
 * without bound.
 */

/** Number of outlines retained before the least recently used one is evicted. */
export const PATH_CACHE_LIMIT = 20_000

/**
 * Builds a `Path2D` from SVG path data. Injectable so the cache can be unit
 * tested in environments (jsdom) that do not implement `Path2D`.
 */
export type PathFactory = (data: string) => Path2D

function browserPathFactory(data: string): Path2D {
  return new Path2D(data)
}

function keyOf(doc: number, gid: number, generation: number): string {
  return `${doc}:${gid}:${generation}`
}

/** An LRU cache of flattened glyph outlines. */
export class PathCache {
  private readonly entries = new Map<string, Path2D | null>()
  private readonly limit: number
  private readonly factory: PathFactory
  private hitCount = 0
  private missCount = 0

  constructor(
    limit = PATH_CACHE_LIMIT,
    factory: PathFactory = browserPathFactory,
  ) {
    this.limit = Math.max(1, limit)
    this.factory = factory
  }

  /** Number of cached entries. */
  get size(): number {
    return this.entries.size
  }

  /** Cache hits since creation; used by the development frame report. */
  get hits(): number {
    return this.hitCount
  }

  /** Cache misses since creation; used by the development frame report. */
  get misses(): number {
    return this.missCount
  }

  /**
   * Returns the outline for one glyph, building it from `data` on a miss.
   * Empty or absent path data caches — and returns — `null` so empty glyphs
   * are not rebuilt on every repaint.
   */
  get(
    doc: number,
    gid: number,
    generation: number,
    data: string | null,
  ): Path2D | null {
    const key = keyOf(doc, gid, generation)
    const cached = this.entries.get(key)
    if (cached !== undefined) {
      this.hitCount += 1
      // Re-insert so the entry becomes the most recently used one.
      this.entries.delete(key)
      this.entries.set(key, cached)
      return cached
    }

    this.missCount += 1
    const path = data ? this.factory(data) : null
    this.entries.set(key, path)
    this.evict()
    return path
  }

  /** Whether an outline for this glyph generation is already cached. */
  has(doc: number, gid: number, generation: number): boolean {
    return this.entries.has(keyOf(doc, gid, generation))
  }

  /**
   * Forgets every generation of the given glyphs in a single pass. Called when
   * a document event reports changed glyphs.
   */
  drop(doc: number, gids: Iterable<number>): void {
    const wanted = new Set(gids)
    if (wanted.size === 0) return
    const prefix = `${doc}:`
    for (const key of [...this.entries.keys()]) {
      if (!key.startsWith(prefix)) continue
      const rest = key.slice(prefix.length)
      const gid = Number(rest.slice(0, rest.indexOf(':')))
      if (wanted.has(gid)) this.entries.delete(key)
    }
  }

  /** Forgets every outline of one document; used when its glyph order changes. */
  clearDoc(doc: number): void {
    const prefix = `${doc}:`
    for (const key of [...this.entries.keys()])
      if (key.startsWith(prefix)) this.entries.delete(key)
  }

  /** Empties the cache entirely. */
  clear(): void {
    this.entries.clear()
  }

  private evict(): void {
    while (this.entries.size > this.limit) {
      const oldest = this.entries.keys().next()
      if (oldest.done) return
      this.entries.delete(oldest.value)
    }
  }
}

/** The process-wide outline cache shared by every glyph grid. */
export const pathCache = new PathCache()
