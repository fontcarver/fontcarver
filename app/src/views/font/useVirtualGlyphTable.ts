import { useCallback, useEffect, useState, type RefObject } from 'react'

import { computeGrid, type VirtualGrid } from './useVirtualGrid'

/** Row height shared by the list's layout and keyboard scrolling. */
export const GLYPH_TABLE_ROW_HEIGHT = 36

/** A single-column virtual list using the grid's common range contract. */
export function useVirtualGlyphTable(
  total: number,
  containerRef: RefObject<HTMLDivElement | null>,
): VirtualGrid {
  const [height, setHeight] = useState(0)
  const [scrollTop, setScrollTop] = useState(0)
  useEffect(() => {
    const element = containerRef.current
    if (!element) return
    const measure = () => setHeight(element.clientHeight)
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [containerRef])

  const onScroll = useCallback(() => {
    setScrollTop(containerRef.current?.scrollTop ?? 0)
  }, [containerRef])

  const scrollTo = useCallback(
    (index: number) => {
      const element = containerRef.current
      if (!element || index < 0) return
      const top = index * GLYPH_TABLE_ROW_HEIGHT
      const bottom = top + GLYPH_TABLE_ROW_HEIGHT
      if (top < element.scrollTop) element.scrollTop = top
      else if (bottom > element.scrollTop + element.clientHeight)
        element.scrollTop = Math.max(0, bottom - element.clientHeight)
      setScrollTop(element.scrollTop)
    },
    [containerRef],
  )

  const metrics = computeGrid({
    total,
    cellSize: GLYPH_TABLE_ROW_HEIGHT,
    gap: 0,
    width: GLYPH_TABLE_ROW_HEIGHT,
    height,
    scrollTop,
  })
  return {
    metrics,
    rows: metrics.rows,
    cols: 1,
    range: [metrics.first, metrics.last],
    scrollTop,
    scrollTo,
    onScroll,
  }
}
