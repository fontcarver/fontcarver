import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { useVirtualGlyphTable } from './useVirtualGlyphTable'

function viewport() {
  const element = document.createElement('div')
  Object.defineProperty(element, 'clientHeight', { value: 720 })
  return { current: element }
}

describe('virtual glyph table', () => {
  it('keeps large lists bounded and brings distant rows into view', () => {
    const containerRef = viewport()
    const { result } = renderHook(() =>
      useVirtualGlyphTable(65_535, containerRef),
    )
    expect(result.current.cols).toBe(1)
    expect(result.current.metrics.last).toBe(21)
    act(() => result.current.scrollTo(60_000))
    expect(result.current.metrics.first).toBeLessThanOrEqual(60_000)
    expect(result.current.metrics.last).toBeGreaterThanOrEqual(60_000)
    expect(
      result.current.metrics.last - result.current.metrics.first,
    ).toBeLessThan(25)
    expect(containerRef.current.scrollTop).toBe(60_001 * 36 - 720)
    act(() => result.current.scrollTo(0))
    expect(containerRef.current.scrollTop).toBe(0)
    expect(result.current.metrics.first).toBe(0)
  })

  it('renders no rows when the current filter is empty', () => {
    const { result } = renderHook(() => useVirtualGlyphTable(0, viewport()))
    expect(result.current.metrics.contentHeight).toBe(0)
    expect(result.current.metrics.last).toBe(-1)
  })
})
