import { expect, it, vi } from 'vitest'
import { cellFor } from '../../test/glyphFixtures'
import { paintCell } from './cellRenderer'

it('paints color-only artwork at the fitted baseline before labels and selection', () => {
  const context = {
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
    measureText: vi.fn(() => ({ width: 8 })),
  } as unknown as CanvasRenderingContext2D
  const color = { paint: vi.fn() }
  const fill = vi.spyOn(context, 'fill')
  const text = vi.spyOn(context, 'fillText')
  paintCell(context, cellFor(0, { is_empty: true, path: null }), {
    size: 96,
    dpr: 1,
    upem: 1000,
    ascender: 800,
    descender: -200,
    labelMode: 'name',
    index: 0,
    selected: true,
    focused: true,
    path: null,
    color,
    fontFamily: 'sans-serif',
    theme: {
      background: '#fff',
      backgroundSelected: '#def',
      glyph: '#123',
      glyphMuted: '#888',
      label: '#456',
      ring: '#00f',
      focusRing: '#00f',
      baseline: '#ccc',
      badge: '#555',
      border: '#ddd',
    },
  })
  expect(color.paint).toHaveBeenCalledWith(context, '#123')
  expect(fill).not.toHaveBeenCalled()
  expect(text).toHaveBeenCalledExactlyOnceWith(
    'A',
    expect.any(Number),
    expect.any(Number),
  )
})
