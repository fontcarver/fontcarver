/**
 * Virtualisation maths for the glyph grid.
 *
 * The scroll container holds one absolutely positioned cell per *visible*
 * index; everything else is a computed offset. The maths is a pure function so
 * it can be asserted against a 65,535-glyph font without a DOM.
 */

import { useCallback, useEffect, useRef, useState, type RefObject } from 'react'

/** Rows rendered above and below the viewport so scrolling never shows a gap. */
export const DEFAULT_OVERSCAN_ROWS = 2

/** Inputs of one grid layout computation. */
export interface GridInput {
  /** Number of cells in the current filter. */
  total: number
  /** Cell edge length in CSS pixels. */
  cellSize: number
  /** Space between cells, and between the cells and the container edge. */
  gap: number
  /** Viewport width in CSS pixels. */
  width: number
  /** Viewport height in CSS pixels. */
  height: number
  /** Current scroll offset in CSS pixels. */
  scrollTop: number
  overscanRows?: number
}

/** The layout of the grid and the slice of it that is currently rendered. */
export interface GridMetrics {
  cols: number
  rows: number
  /** Distance between the top-left corners of two neighbouring cells. */
  pitch: number
  /** Height of the scrollable content in CSS pixels. */
  contentHeight: number
  /** First and last rendered row, inclusive. */
  firstRow: number
  lastRow: number
  /** First and last rendered cell index, inclusive; `last` is -1 when empty. */
  first: number
  last: number
}

/** Position of one cell inside the scroll content. */
export interface CellBox {
  x: number
  y: number
  size: number
}

/** Computes the grid layout and the visible index range. */
export function computeGrid(input: GridInput): GridMetrics {
  const gap = Math.max(0, input.gap)
  const cellSize = Math.max(1, input.cellSize)
  const pitch = cellSize + gap
  const cols = Math.max(1, Math.floor((input.width - gap) / pitch))
  const total = Math.max(0, input.total)
  const rows = Math.ceil(total / cols)
  const contentHeight = rows === 0 ? 0 : rows * pitch + gap
  const overscan = input.overscanRows ?? DEFAULT_OVERSCAN_ROWS

  if (rows === 0) {
    return {
      cols,
      rows,
      pitch,
      contentHeight,
      firstRow: 0,
      lastRow: -1,
      first: 0,
      last: -1,
    }
  }

  // A row is visible when its box overlaps the viewport. Row `r` spans
  // `gap + r * pitch` to `gap + r * pitch + cellSize`, so a row whose top edge
  // sits exactly on the viewport's bottom edge is *not* visible; using
  // `floor` there would render one row too many below the fold.
  const visibleFirst = Math.floor(
    (input.scrollTop - gap - cellSize) / pitch + 1,
  )
  const visibleLast =
    Math.ceil((input.scrollTop + input.height - gap) / pitch) - 1
  const firstRow = Math.min(
    rows - 1,
    Math.max(0, Math.max(0, visibleFirst) - overscan),
  )
  const lastRow = Math.min(rows - 1, Math.max(firstRow, visibleLast + overscan))
  const first = firstRow * cols
  const last = Math.min(total - 1, (lastRow + 1) * cols - 1)
  return { cols, rows, pitch, contentHeight, firstRow, lastRow, first, last }
}

/** Where one cell sits inside the scroll content. */
export function cellBox(
  index: number,
  metrics: GridMetrics,
  cellSize: number,
  gap: number,
): CellBox {
  const row = Math.floor(index / metrics.cols)
  const col = index - row * metrics.cols
  return {
    x: gap + col * metrics.pitch,
    y: gap + row * metrics.pitch,
    size: cellSize,
  }
}

/** Every cell index whose box intersects `rect`, in ascending order. */
export function indicesInRect(
  rect: { x: number; y: number; width: number; height: number },
  metrics: GridMetrics,
  cellSize: number,
  gap: number,
  total: number,
): number[] {
  const left = Math.min(rect.x, rect.x + rect.width)
  const right = Math.max(rect.x, rect.x + rect.width)
  const top = Math.min(rect.y, rect.y + rect.height)
  const bottom = Math.max(rect.y, rect.y + rect.height)

  const firstRow = Math.max(0, Math.floor((top - gap) / metrics.pitch))
  const lastRow = Math.min(
    metrics.rows - 1,
    Math.floor((bottom - gap) / metrics.pitch),
  )
  const firstCol = Math.max(0, Math.floor((left - gap) / metrics.pitch))
  const lastCol = Math.min(
    metrics.cols - 1,
    Math.floor((right - gap) / metrics.pitch),
  )

  const found: number[] = []
  for (let row = firstRow; row <= lastRow; row += 1) {
    const y = gap + row * metrics.pitch
    if (y > bottom || y + cellSize < top) continue
    for (let col = firstCol; col <= lastCol; col += 1) {
      const x = gap + col * metrics.pitch
      if (x > right || x + cellSize < left) continue
      const index = row * metrics.cols + col
      if (index < total) found.push(index)
    }
  }
  return found
}

/** Options of {@link useVirtualGrid}. */
export interface VirtualGridOptions {
  total: number
  cellSize: number
  gap: number
  containerRef: RefObject<HTMLElement | null>
  overscanRows?: number
}

/** The grid state a component needs to render and scroll the virtualised list. */
export interface VirtualGrid {
  metrics: GridMetrics
  rows: number
  cols: number
  range: [number, number]
  /** Scrolls the container so that the cell at `index` is fully visible. */
  scrollTo: (index: number) => void
  /** Scroll handler to attach to the container. */
  onScroll: () => void
}

/**
 * Tracks the container size and scroll offset and derives the rendered range.
 * The scroll offset is quantised to whole rows, so a re-render happens once per
 * row crossed rather than once per scrolled pixel.
 */
export function useVirtualGrid({
  total,
  cellSize,
  gap,
  containerRef,
  overscanRows = DEFAULT_OVERSCAN_ROWS,
}: VirtualGridOptions): VirtualGrid {
  const [viewport, setViewport] = useState({ width: 0, height: 0 })
  const [scrollTop, setScrollTop] = useState(0)
  const pitch = Math.max(1, cellSize) + Math.max(0, gap)
  const scrollRef = useRef(0)

  useEffect(() => {
    const element = containerRef.current
    if (!element) return
    const measure = () =>
      setViewport({
        width: element.clientWidth,
        height: element.clientHeight,
      })
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [containerRef])

  const onScroll = useCallback(() => {
    const element = containerRef.current
    if (!element) return
    scrollRef.current = element.scrollTop
    const quantised = Math.floor(element.scrollTop / pitch) * pitch
    setScrollTop((current) => (current === quantised ? current : quantised))
  }, [containerRef, pitch])

  const metrics = computeGrid({
    total,
    cellSize,
    gap,
    width: viewport.width,
    height: viewport.height,
    scrollTop,
    overscanRows,
  })

  const scrollTo = useCallback(
    (index: number) => {
      const element = containerRef.current
      if (!element || index < 0) return
      const cols = Math.max(
        1,
        Math.floor((element.clientWidth - gap) / (cellSize + gap)),
      )
      const row = Math.floor(index / cols)
      const top = gap + row * (cellSize + gap)
      const bottom = top + cellSize
      if (top < element.scrollTop) element.scrollTop = Math.max(0, top - gap)
      else if (bottom > element.scrollTop + element.clientHeight)
        element.scrollTop = bottom - element.clientHeight + gap
    },
    [cellSize, containerRef, gap],
  )

  return {
    metrics,
    rows: metrics.rows,
    cols: metrics.cols,
    range: [metrics.first, metrics.last],
    scrollTo,
    onScroll,
  }
}
