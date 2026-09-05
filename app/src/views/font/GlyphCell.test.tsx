import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { emBoxFor } from '../../canvas/glyphPaint'
import { cellFor } from '../../test/glyphFixtures'

import type { CellTheme } from './cellRenderer'
import { GlyphCell } from './GlyphCell'

const THEME: CellTheme = {
  background: '#fff',
  backgroundSelected: '#dde7f2',
  glyph: '#17171a',
  glyphMuted: '#6b6b74',
  label: '#5a5a63',
  ring: '#3a6ea5',
  focusRing: '#3a6ea5',
  baseline: '#cfcfd3',
  badge: '#3a6ea5',
  border: '#e2e2e5',
}

function props(cell: ReturnType<typeof cellFor> | undefined) {
  return {
    doc: 1,
    cell,
    index: 0,
    total: 600,
    x: 8,
    y: 8,
    size: 64,
    dpr: 2,
    em: emBoxFor(1000, 800, -200),
    labelMode: 'name' as const,
    selected: false,
    focused: false,
    theme: THEME,
    fontFamily: 'system-ui',
  }
}

function canvasIn(container: HTMLElement): HTMLCanvasElement {
  const canvas = container.querySelector('canvas')
  if (!canvas) throw new Error('the cell rendered no canvas')
  return canvas
}

describe('GlyphCell', () => {
  it('releases the backing store of a cell that arrived after the first render', () => {
    // The cell starts pending, so no canvas exists while the component mounts.
    const { container, rerender, unmount } = render(
      <GlyphCell {...props(undefined)} />,
    )
    expect(container.querySelector('canvas')).toBeNull()

    rerender(<GlyphCell {...props(cellFor(0))} />)
    const canvas = canvasIn(container)
    expect(canvas.width).toBe(128)
    expect(canvas.height).toBe(128)

    unmount()

    expect(canvas.width).toBe(0)
    expect(canvas.height).toBe(0)
  })

  it('releases the backing store of a cell present from the start', () => {
    const { container, unmount } = render(<GlyphCell {...props(cellFor(3))} />)
    const canvas = canvasIn(container)
    expect(canvas.width).toBe(128)

    unmount()

    expect(canvas.width).toBe(0)
    expect(canvas.height).toBe(0)
  })

  it('releases the backing store when a loaded cell goes back to pending', () => {
    const { container, rerender } = render(<GlyphCell {...props(cellFor(4))} />)
    const canvas = canvasIn(container)
    expect(canvas.width).toBe(128)

    rerender(<GlyphCell {...props(undefined)} />)

    expect(container.querySelector('canvas')).toBeNull()
    expect(canvas.width).toBe(0)
  })

  it('repaints a cell whose page was invalidated and came back', () => {
    // An edit that invalidates the page cache blanks the cell for a frame:
    // the canvas unmounts, and its ref cleanup must not null the handle the
    // *replacement* canvas has already installed, or the cell that comes back
    // never paints again.
    const { container, rerender } = render(<GlyphCell {...props(cellFor(6))} />)
    const first = canvasIn(container)
    expect(first.width).toBe(128)

    rerender(<GlyphCell {...props(undefined)} />)
    rerender(<GlyphCell {...props(cellFor(6))} />)

    const second = canvasIn(container)
    expect(second.width).toBe(128)
    expect(second.height).toBe(128)
  })

  it('keeps the canvas across a repaint', () => {
    const { container, rerender } = render(<GlyphCell {...props(cellFor(5))} />)
    const canvas = canvasIn(container)

    rerender(<GlyphCell {...props(cellFor(5))} selected />)

    expect(canvasIn(container)).toBe(canvas)
    expect(canvas.width).toBe(128)
  })
})
