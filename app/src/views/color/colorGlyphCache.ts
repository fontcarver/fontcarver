import type { ColorGlyphResource } from '../../canvas/colorGlyph'

interface Entry {
  refs: number
  started: boolean
  value?: ColorGlyphResource | null
  result: Promise<ColorGlyphResource | null>
  resolve(value: ColorGlyphResource | null): void
}

/** Bounded leased color resources; queued work never outlives its consumers. */
export class ColorGlyphCache {
  private readonly entries = new Map<string, Entry>()
  private active = 0
  private disposed = false

  constructor(
    private readonly load: (
      glyph: string,
    ) => Promise<ColorGlyphResource | null>,
    private readonly capacity = 128,
    private readonly concurrency = 4,
  ) {}

  acquire(glyph: string): {
    result: Promise<ColorGlyphResource | null>
    release(): void
  } {
    if (this.disposed) return { result: Promise.resolve(null), release() {} }
    let entry = this.entries.get(glyph)
    if (!entry) {
      if (this.entries.size >= this.capacity) {
        const unused = [...this.entries].find(([, item]) => item.refs === 0)
        if (unused) {
          this.entries.delete(unused[0])
          unused[1].value?.dispose()
          unused[1].resolve(null)
        }
      }
      if (this.entries.size >= this.capacity)
        return { result: Promise.resolve(null), release() {} }
      let resolve!: Entry['resolve']
      const result = new Promise<ColorGlyphResource | null>((done) => {
        resolve = done
      })
      entry = { refs: 0, started: false, result, resolve }
      this.entries.set(glyph, entry)
    }
    entry.refs += 1
    this.pump()
    let released = false
    return {
      result: entry.result,
      release: () => {
        if (released) return
        released = true
        entry.refs -= 1
        if (entry.refs === 0 && !entry.started) {
          this.entries.delete(glyph)
          entry.resolve(null)
        }
      },
    }
  }

  private pump(): void {
    if (this.disposed) return
    for (const [glyph, entry] of this.entries) {
      if (this.active >= this.concurrency) return
      if (entry.started || entry.refs === 0) continue
      entry.started = true
      this.active += 1
      let loaded: Promise<ColorGlyphResource | null>
      try {
        loaded = this.load(glyph)
      } catch {
        loaded = Promise.resolve(null)
      }
      void loaded
        .catch(() => null)
        .then((value) => {
          this.active -= 1
          if (this.disposed || this.entries.get(glyph) !== entry) {
            value?.dispose()
            entry.resolve(null)
          } else {
            entry.value = value
            entry.resolve(value)
          }
          this.pump()
        })
    }
  }

  dispose(): void {
    if (this.disposed) return
    this.disposed = true
    for (const entry of this.entries.values()) {
      entry.value?.dispose()
      if (!entry.started) entry.resolve(null)
    }
    this.entries.clear()
  }
}
