import { describe, expect, it } from 'vitest'

import { cellBox, computeGrid, indicesInRect } from './useVirtualGrid'

const CJK = {
  total: 65_535,
  cellSize: 64,
  gap: 8,
  width: 1200,
  height: 800,
  overscanRows: 2,
}

describe('computeGrid', () => {
  it('lays out a 65,535 glyph font in a 1200x800 viewport', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 0 })

    expect(metrics.pitch).toBe(72)
    expect(metrics.cols).toBe(16)
    expect(metrics.rows).toBe(4096)
    expect(metrics.contentHeight).toBe(4096 * 72 + 8)
    expect(metrics.firstRow).toBe(0)
    // Rows 0-10 are visible (row 11 starts exactly on the fold), plus two
    // rows of overscan below.
    expect(metrics.lastRow).toBe(12)
    expect([metrics.first, metrics.last]).toEqual([0, 207])
  })

  it('renders exactly two overscan rows past the last visible one', () => {
    // 800 px of viewport, 8 px of leading gap, 72 px pitch: the last row that
    // overlaps the viewport is row 10, whose box is 728-792.
    const metrics = computeGrid({ ...CJK, scrollTop: 0 })
    const lastVisibleTop = 8 + 10 * 72

    expect(lastVisibleTop).toBeLessThan(800)
    expect(8 + 11 * 72).toBe(800)
    expect(metrics.lastRow - 10).toBe(2)
  })

  it('counts a row as visible as soon as one pixel of it is on screen', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 8, overscanRows: 0 })

    // Scrolled by 8 px the viewport ends at 808, so row 11 (800-864) shows.
    expect(metrics.firstRow).toBe(0)
    expect(metrics.lastRow).toBe(11)
  })

  it('drops a row whose bottom edge has just left the viewport', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 72, overscanRows: 0 })

    // Row 0 spans 8-72, so at scrollTop 72 it is exactly gone.
    expect(metrics.firstRow).toBe(1)
  })

  it('renders only the visible rows plus two rows of overscan when scrolled', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 100_000 })

    expect(metrics.firstRow).toBe(1386)
    expect(metrics.lastRow).toBe(1401)
    expect([metrics.first, metrics.last]).toEqual([22_176, 22_431])
    expect(metrics.last - metrics.first + 1).toBe(256)
  })

  it('clamps the last range to the final, partly filled row', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 294_920 })

    expect(metrics.lastRow).toBe(4095)
    expect(metrics.last).toBe(65_534)
  })

  it('keeps at least one column in a viewport narrower than a cell', () => {
    const metrics = computeGrid({ ...CJK, width: 20, scrollTop: 0 })

    expect(metrics.cols).toBe(1)
    expect(metrics.rows).toBe(65_535)
  })

  it('reports an empty range for a filter that matches nothing', () => {
    const metrics = computeGrid({ ...CJK, total: 0, scrollTop: 0 })

    expect(metrics.rows).toBe(0)
    expect(metrics.contentHeight).toBe(0)
    expect(metrics.last).toBe(-1)
  })
})

describe('cellBox', () => {
  it('places cells on the pitch grid inside the container padding', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 0 })

    expect(cellBox(0, metrics, 64, 8)).toEqual({ x: 8, y: 8, size: 64 })
    expect(cellBox(17, metrics, 64, 8)).toEqual({ x: 80, y: 80, size: 64 })
  })
})

describe('indicesInRect', () => {
  it('collects every cell a rubber band touches', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 0 })

    const indices = indicesInRect(
      { x: 10, y: 10, width: 140, height: 80 },
      metrics,
      64,
      8,
      CJK.total,
    )

    expect(indices).toEqual([0, 1, 16, 17])
  })

  it('normalises a rectangle dragged up and to the left', () => {
    const metrics = computeGrid({ ...CJK, scrollTop: 0 })

    const indices = indicesInRect(
      { x: 150, y: 90, width: -140, height: -80 },
      metrics,
      64,
      8,
      CJK.total,
    )

    expect(indices).toEqual([0, 1, 16, 17])
  })

  it('never returns an index past the end of the filter', () => {
    const metrics = computeGrid({ ...CJK, total: 3, scrollTop: 0 })

    const indices = indicesInRect(
      { x: 0, y: 0, width: 1200, height: 800 },
      metrics,
      64,
      8,
      3,
    )

    expect(indices).toEqual([0, 1, 2])
  })
})
