/**
 * Page scheduler for the glyph grid.
 *
 * The grid never asks for more than the pages that overlap the rendered range.
 * Requests are debounced by one frame so a fast scroll issues one burst instead
 * of one request per row, and every response carries the request epoch it was
 * issued under: a filter, sort or master change bumps the epoch and the answers
 * that were already in flight are dropped instead of poisoning the new view.
 */

import type { GlyphCell, GlyphListResult } from '../../api'

/** Cells fetched per request. */
export const GLYPH_PAGE_SIZE = 300

/** Debounce applied to visible-range changes, in milliseconds. */
export const PAGE_DEBOUNCE_MS = 16

/** Construction options of {@link GlyphPager}. */
export interface GlyphPagerOptions {
  /** Issues one `glyphs.list` request. */
  fetchPage: (offset: number, limit: number) => Promise<GlyphListResult>
  /** Called whenever cached cells or the total changed. */
  onChange: () => void
  /** Called when a page request fails. */
  onError?: (error: unknown) => void
  pageSize?: number
  debounceMs?: number
}

/** Keeps the cells of the visible range of one filtered, sorted glyph list. */
export class GlyphPager {
  private readonly options: Required<Omit<GlyphPagerOptions, 'onError'>> &
    Pick<GlyphPagerOptions, 'onError'>
  private readonly cells = new Map<number, GlyphCell>()
  private readonly loaded = new Set<number>()
  private readonly pending = new Set<number>()
  private range: [number, number] = [0, -1]
  private timer: ReturnType<typeof setTimeout> | undefined
  private epoch = 0
  private totalValue: number | null = null

  constructor(options: GlyphPagerOptions) {
    this.options = {
      pageSize: GLYPH_PAGE_SIZE,
      debounceMs: PAGE_DEBOUNCE_MS,
      ...options,
    }
  }

  /** Number of glyphs matching the current filter, or `null` until known. */
  get total(): number | null {
    return this.totalValue
  }

  /** The cell at one position of the filtered, sorted order, when loaded. */
  cell(index: number): GlyphCell | undefined {
    return this.cells.get(index)
  }

  /** Every loaded cell in the given inclusive index range. */
  cellsIn(first: number, last: number): GlyphCell[] {
    const found: GlyphCell[] = []
    for (let index = first; index <= last; index += 1) {
      const cell = this.cells.get(index)
      if (cell) found.push(cell)
    }
    return found
  }

  /**
   * Declares the rendered range. Scheduling is debounced, so a scroll that
   * crosses many rows produces a single burst of page requests.
   */
  setRange(first: number, last: number): void {
    this.range = [first, last]
    if (this.timer !== undefined) clearTimeout(this.timer)
    this.timer = setTimeout(() => {
      this.timer = undefined
      this.request()
    }, this.options.debounceMs)
  }

  /** Issues the outstanding requests immediately, skipping the debounce. */
  flush(): void {
    if (this.timer !== undefined) {
      clearTimeout(this.timer)
      this.timer = undefined
    }
    this.request()
  }

  /**
   * Discards every cached cell and invalidates the responses in flight. Called
   * when the filter, sort, master or glyph order changes.
   */
  reset(): void {
    this.epoch += 1
    this.cells.clear()
    this.loaded.clear()
    this.pending.clear()
    this.totalValue = null
    this.options.onChange()
  }

  /** Forgets the cells of the given glyphs so their pages are fetched again. */
  invalidate(gids: Iterable<number>): void {
    const wanted = new Set(gids)
    if (wanted.size === 0) return
    let changed = false
    for (const [index, cell] of [...this.cells]) {
      if (!wanted.has(cell.gid)) continue
      this.cells.delete(index)
      this.loaded.delete(Math.floor(index / this.options.pageSize))
      changed = true
    }
    if (!changed) return
    this.options.onChange()
    this.setRange(this.range[0], this.range[1])
  }

  /**
   * Stops the debounce timer and drops every response still in flight, keeping
   * the cells already loaded. The grid calls this when it unmounts; a mount
   * that follows (React's development double-invoke) simply asks again.
   */
  cancel(): void {
    this.epoch += 1
    this.pending.clear()
    if (this.timer !== undefined) clearTimeout(this.timer)
    this.timer = undefined
  }

  private request(): void {
    const [first, last] = this.range
    if (last < first) return
    const { pageSize } = this.options
    const firstPage = Math.floor(first / pageSize)
    const lastPage = Math.floor(last / pageSize)
    for (let page = firstPage; page <= lastPage; page += 1) {
      if (this.loaded.has(page) || this.pending.has(page)) continue
      const offset = page * pageSize
      if (this.totalValue !== null && offset >= this.totalValue) continue
      this.pending.add(page)
      const epoch = this.epoch
      this.options
        .fetchPage(offset, pageSize)
        .then((result) => this.receive(page, offset, epoch, result))
        .catch((error: unknown) => {
          if (epoch !== this.epoch) return
          this.pending.delete(page)
          this.options.onError?.(error)
        })
    }
  }

  private receive(
    page: number,
    offset: number,
    epoch: number,
    result: GlyphListResult,
  ): void {
    if (epoch !== this.epoch) return
    this.pending.delete(page)
    this.loaded.add(page)
    this.totalValue = result.total
    result.items.forEach((cell, position) =>
      this.cells.set(offset + position, cell),
    )
    this.options.onChange()
  }
}
