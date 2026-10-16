/**
 * Page scheduler for the glyph grid.
 *
 * The grid never asks for more than the pages that overlap the rendered range.
 * Requests are debounced by one frame so a fast scroll issues one burst instead
 * of one request per row, and every response is stamped twice:
 *
 * - with a **query epoch**, bumped when the filter, sort or master changes, so
 *   answers for the previous list are dropped instead of poisoning the new one;
 * - with the **document generation** current when the request left, so an
 *   answer computed before an edit is dropped rather than reinstating cells the
 *   edit has already invalidated.
 *
 * Loaded pages are held in an LRU bounded by {@link MAX_CACHED_PAGES}, so
 * traversing a 65k-glyph font does not accumulate the whole font (each cell
 * carries its flattened outline as SVG path data).
 */

import type { GlyphCell, GlyphListResult } from '../../api'

/** Cells fetched per request. */
export const GLYPH_PAGE_SIZE = 300

/** Debounce applied to visible-range changes, in milliseconds. */
export const PAGE_DEBOUNCE_MS = 16

/** Pages retained before the least recently used one outside the view is dropped. */
export const MAX_CACHED_PAGES = 40

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
  maxPages?: number
}

/** Keeps the cells of the visible range of one filtered, sorted glyph list. */
export class GlyphPager {
  private readonly options: Required<Omit<GlyphPagerOptions, 'onError'>> &
    Pick<GlyphPagerOptions, 'onError'>
  private readonly cells = new Map<number, GlyphCell>()
  /** Loaded pages in least-recently-used order. */
  private readonly loaded = new Map<number, true>()
  private readonly pending = new Set<number>()
  private range: [number, number] = [0, -1]
  private timer: ReturnType<typeof setTimeout> | undefined
  private epoch = 0
  private generationValue = 0
  private totalValue: number | null = null

  constructor(options: GlyphPagerOptions) {
    this.options = {
      pageSize: GLYPH_PAGE_SIZE,
      debounceMs: PAGE_DEBOUNCE_MS,
      maxPages: MAX_CACHED_PAGES,
      ...options,
    }
  }

  /** Number of glyphs matching the current filter, or `null` until known. */
  get total(): number | null {
    return this.totalValue
  }

  /** Document generation the pager currently accepts answers for. */
  get generation(): number {
    return this.generationValue
  }

  /** Pages currently held; bounded by the `maxPages` option. */
  get pageCount(): number {
    return this.loaded.size
  }

  /** Cells currently held, across every retained page. */
  get cellCount(): number {
    return this.cells.size
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
   * crosses many rows produces a single burst of page requests. The pages the
   * range covers become the most recently used ones, so eviction never takes a
   * page that is on screen.
   */
  setRange(first: number, last: number): void {
    this.range = [first, last]
    if (last >= first) {
      const { pageSize } = this.options
      for (
        let page = Math.floor(first / pageSize);
        page <= Math.floor(last / pageSize);
        page += 1
      )
        this.touch(page)
    }
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
   * Raises the document generation the pager accepts answers for. Requests
   * already in flight were computed against the previous state, so they stop
   * counting as pending and their answers are discarded on arrival.
   */
  setGeneration(generation: number): void {
    if (generation <= this.generationValue) return
    this.generationValue = generation
    this.pending.clear()
  }

  /**
   * Discards every cached cell, the total and the responses in flight. Called
   * when the filter, sort, master or glyph order changes, and whenever an edit
   * could have changed which glyphs match or in what order.
   */
  reset(): void {
    this.epoch += 1
    this.cells.clear()
    this.loaded.clear()
    this.pending.clear()
    this.totalValue = null
    this.options.onChange()
  }

  /**
   * Marks every page that held one of these glyphs stale: its cells are
   * dropped (so nothing pre-edit stays on screen) and the page is fetched
   * again the next time it is in range.
   */
  invalidate(gids: Iterable<number>): void {
    const wanted = new Set(gids)
    if (wanted.size === 0) return
    const { pageSize } = this.options
    const stale = new Set<number>()
    for (const [index, cell] of this.cells)
      if (wanted.has(cell.gid)) stale.add(Math.floor(index / pageSize))
    if (stale.size === 0) return
    for (const page of stale) this.dropPage(page)
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

  private touch(page: number): void {
    if (!this.loaded.delete(page)) return
    this.loaded.set(page, true)
  }

  private dropPage(page: number): void {
    this.loaded.delete(page)
    const start = page * this.options.pageSize
    for (let index = start; index < start + this.options.pageSize; index += 1)
      this.cells.delete(index)
  }

  /** Drops least recently used pages outside the rendered range past the cap. */
  private evict(): void {
    const { maxPages, pageSize } = this.options
    if (this.loaded.size <= maxPages) return
    const [first, last] = this.range
    const keepFirst = Math.floor(first / pageSize)
    const keepLast = Math.floor(last / pageSize)
    for (const page of [...this.loaded.keys()]) {
      if (this.loaded.size <= maxPages) break
      if (last >= first && page >= keepFirst && page <= keepLast) continue
      this.dropPage(page)
    }
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
      const generation = this.generationValue
      this.options
        .fetchPage(offset, pageSize)
        .then((result) => this.receive(page, offset, epoch, generation, result))
        .catch((error: unknown) => {
          if (epoch !== this.epoch || generation < this.generationValue) return
          this.pending.delete(page)
          this.options.onError?.(error)
        })
    }
  }

  private receive(
    page: number,
    offset: number,
    epoch: number,
    generation: number,
    result: GlyphListResult,
  ): void {
    if (epoch !== this.epoch) return
    if (generation < this.generationValue) {
      // Computed before the edit that raised the generation; ask again.
      this.options.onChange()
      return
    }
    this.pending.delete(page)
    this.loaded.delete(page)
    this.loaded.set(page, true)
    this.totalValue = result.total
    result.items.forEach((cell, position) =>
      this.cells.set(offset + position, cell),
    )
    this.evict()
    this.options.onChange()
  }
}

/**
 * Whether a document change invalidates the *shape* of the list rather than
 * individual cells: the order or the membership of the filter can then have
 * changed anywhere, so only a full reload restores the truth.
 *
 * Any sort other than the font's own glyph order is derived from a glyph
 * attribute an edit may have altered (name, Unicode, width), and any active
 * filter decides membership from attributes an edit may have altered, so both
 * force the reload.
 */
export function requiresFullReload(
  change: {
    glyph_order: boolean
    glyphs: readonly number[]
    font_info?: boolean
  },
  sort: string,
  filtered: boolean,
): boolean {
  if (change.glyph_order || (sort === 'Cid' && change.font_info)) return true
  if (change.glyphs.length === 0) return false
  return sort !== 'Order' || filtered
}

/**
 * The cells of one inclusive index window, holes included.
 *
 * The pager's cells are a mutable cache, so a caller that renders them takes
 * this snapshot together with the `version` the pager last reported: a new
 * version yields a new array. That is what tells React — and automatic
 * memoisation, which may otherwise treat a repeated `cell` call with the same
 * index as unchanged — that the window must be read again.
 */
export function cellWindow(
  pager: GlyphPager,
  first: number,
  last: number,
  version: number,
): readonly (GlyphCell | undefined)[] {
  void version
  const cells: (GlyphCell | undefined)[] = []
  for (let index = first; index <= last; index += 1)
    cells.push(pager.cell(index))
  return cells
}
