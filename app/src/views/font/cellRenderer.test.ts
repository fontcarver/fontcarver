import { describe, expect, it, vi } from 'vitest'

import type { GlyphCell } from '../../api'

import {
  cellLabel,
  formatUnicode,
  paintCell,
  type CellTheme,
  type PaintCellOptions,
} from './cellRenderer'

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

/** A 2D context double recording every call the renderer makes. */
function mockContext() {
  return {
    setTransform: vi.fn(),
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    strokeRect: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    rect: vi.fn(),
    clip: vi.fn(),
    save: vi.fn(),
    restore: vi.fn(),
    translate: vi.fn(),
    scale: vi.fn(),
    setLineDash: vi.fn(),
    fillText: vi.fn(),
    measureText: vi.fn((text: string) => ({ width: text.length * 6 })),
    font: '',
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
    textAlign: 'start',
    textBaseline: 'alphabetic',
  }
}

function makeCell(overrides: Partial<GlyphCell> = {}): GlyphCell {
  return {
    gid: 4,
    name: 'A',
    unicodes: [0x41],
    width: 1479,
    bbox: [10, 0, 1400, 1493],
    path: 'M0 0L100 0L100 100Z',
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 0,
    ...overrides,
  }
}

function options(overrides: Partial<PaintCellOptions> = {}): PaintCellOptions {
  return {
    size: 64,
    dpr: 2,
    upem: 2048,
    ascender: 1556,
    descender: -492,
    labelMode: 'name',
    index: 0,
    selected: false,
    focused: false,
    theme: THEME,
    path: { id: 'outline' } as unknown as Path2D,
    fontFamily: 'system-ui',
    ...overrides,
  }
}

describe('paintCell', () => {
  it('scales the context by the device pixel ratio and fills the outline', () => {
    const ctx = mockContext()

    paintCell(ctx as unknown as CanvasRenderingContext2D, makeCell(), options())

    expect(ctx.setTransform).toHaveBeenCalledWith(2, 0, 0, 2, 0, 0)
    expect(ctx.clearRect).toHaveBeenCalledWith(0, 0, 64, 64)
    expect(ctx.fill).toHaveBeenCalledTimes(1)
    expect(ctx.fill.mock.calls[0]?.[0]).toEqual({ id: 'outline' })
    // The glyph is drawn y-flipped from the baseline.
    const [scaleX, scaleY] = ctx.scale.mock.calls[0] ?? []
    expect(scaleX).toBeGreaterThan(0)
    expect(scaleY).toBe(-(scaleX as number))
  })

  it('draws the glyph name under the glyph', () => {
    const ctx = mockContext()

    paintCell(ctx as unknown as CanvasRenderingContext2D, makeCell(), options())

    expect(ctx.fillText).toHaveBeenCalledTimes(1)
    expect(ctx.fillText.mock.calls[0]?.[0]).toBe('A')
  })

  it('marks an empty glyph with a dashed box instead of an outline', () => {
    const ctx = mockContext()

    paintCell(
      ctx as unknown as CanvasRenderingContext2D,
      makeCell({ is_empty: true, path: '' }),
      options({ path: null }),
    )

    expect(ctx.fill).not.toHaveBeenCalled()
    expect(ctx.setLineDash).toHaveBeenCalledWith([2, 2])
  })

  it('paints the selection ring and the selected background', () => {
    const plain = mockContext()
    const selected = mockContext()

    paintCell(
      plain as unknown as CanvasRenderingContext2D,
      makeCell(),
      options(),
    )
    paintCell(
      selected as unknown as CanvasRenderingContext2D,
      makeCell(),
      options({ selected: true }),
    )

    expect(plain.strokeRect).toHaveBeenCalledWith(0.5, 0.5, 63, 63)
    expect(selected.strokeRect).toHaveBeenCalledWith(0.75, 0.75, 62.5, 62.5)
    expect(selected.fillRect.mock.calls[0]).toEqual([0, 0, 64, 64])
  })

  it('paints the colour-label stripe along the top of the cell', () => {
    const ctx = mockContext()

    paintCell(
      ctx as unknown as CanvasRenderingContext2D,
      makeCell({ color_label: 0 }),
      options(),
    )

    expect(ctx.fillRect).toHaveBeenCalledWith(0, 0, 64, 3)
  })

  it('hatches a glyph that does not export', () => {
    const ctx = mockContext()

    paintCell(
      ctx as unknown as CanvasRenderingContext2D,
      makeCell({ export: false }),
      options(),
    )

    expect(ctx.clip).toHaveBeenCalled()
    expect(ctx.stroke).toHaveBeenCalled()
  })

  it('still labels a 32 px cell, with smaller type', () => {
    const ctx = mockContext()

    paintCell(
      ctx as unknown as CanvasRenderingContext2D,
      makeCell(),
      options({ size: 32 }),
    )

    expect(ctx.fillText).toHaveBeenCalledTimes(1)
    expect(ctx.fillText.mock.calls[0]?.[0]).toBe('A')
    expect(ctx.font).toBe('8px system-ui')
  })

  it('drops the label only in cells too small to read one', () => {
    const ctx = mockContext()

    paintCell(
      ctx as unknown as CanvasRenderingContext2D,
      makeCell(),
      options({ size: 24 }),
    )

    expect(ctx.fillText).not.toHaveBeenCalled()
  })

  it('labels index mode in a small cell too', () => {
    const ctx = mockContext()

    paintCell(
      ctx as unknown as CanvasRenderingContext2D,
      makeCell(),
      options({ size: 32, labelMode: 'index', index: 12 }),
    )

    expect(ctx.fillText.mock.calls[0]?.[0]).toBe('12')
  })
})

describe('cellLabel', () => {
  it('formats the three label modes', () => {
    const cell = makeCell()

    expect(cellLabel(cell, 'name', 41)).toBe('A')
    expect(cellLabel(cell, 'unicode', 41)).toBe('U+0041')
    expect(cellLabel(cell, 'index', 41)).toBe('41')
    expect(cellLabel(makeCell({ unicodes: [] }), 'unicode', 0)).toBe('—')
  })

  it('pads short scalars to four digits and keeps long ones intact', () => {
    expect(formatUnicode(0x41)).toBe('U+0041')
    expect(formatUnicode(0x1f600)).toBe('U+1F600')
  })
})
