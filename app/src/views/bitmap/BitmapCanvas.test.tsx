import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { BitmapCanvas } from './BitmapCanvas'
import { blank, put, sample } from './pixels'
import type { BitmapGlyph } from '../../api'

vi.mock('../../canvas/bitmapRenderer', () => ({ renderBitmap: vi.fn() }))
const glyph = (): BitmapGlyph => ({
  bitmap: blank(3, 3, 1),
  advance: 4,
  vertical_advance: null,
  refs: [],
})
function mount(tool: 'pencil' | 'eyedropper' | 'hand', value = glyph()) {
  const onEdit = vi.fn(),
    onCommit = vi.fn(),
    onPickInk = vi.fn()
  render(
    <BitmapCanvas
      value={value}
      zoom={10}
      tool={tool}
      ink={1}
      disabled={false}
      onSelect={vi.fn()}
      onEdit={onEdit}
      onCommit={onCommit}
      onPickInk={onPickInk}
      upm={1000}
      ppem={20}
    />,
  )
  const canvas = screen.getByLabelText(
    'Bitmap pixel canvas',
  ) as HTMLCanvasElement
  canvas.getBoundingClientRect = () => ({
    left: 0,
    top: 0,
    width: 60,
    height: 50,
    right: 60,
    bottom: 50,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  })
  canvas.setPointerCapture = vi.fn()
  return { canvas, onEdit, onCommit, onPickInk }
}

beforeEach(() => {
  vi.stubGlobal('PointerEvent', MouseEvent)
})
describe('bitmap canvas gestures', () => {
  it('keeps the pencil erase choice for the whole drag and commits once', () => {
    const value = glyph()
    put(value.bitmap, 0, 0, 1)
    put(value.bitmap, 1, 0, 1)
    const { canvas, onEdit, onCommit } = mount('pencil', value)
    fireEvent.pointerDown(canvas, { button: 0, clientX: 15, clientY: 15 })
    fireEvent.pointerMove(canvas, { clientX: 35, clientY: 15 })
    fireEvent.pointerUp(canvas)
    const edited = onEdit.mock.lastCall?.[0] as BitmapGlyph
    expect([0, 1, 2].map((x) => sample(edited.bitmap, x, 0))).toEqual([0, 0, 0])
    expect(onCommit).toHaveBeenCalledTimes(1)
  })
  it('picks exact ink without writing or committing the glyph', () => {
    const value = glyph()
    put(value.bitmap, 0, 0, 1)
    const { canvas, onEdit, onCommit, onPickInk } = mount('eyedropper', value)
    fireEvent.pointerDown(canvas, { button: 0, clientX: 15, clientY: 15 })
    fireEvent.pointerUp(canvas)
    expect(onPickInk).toHaveBeenCalledWith(1)
    expect(onEdit).not.toHaveBeenCalled()
    expect(onCommit).not.toHaveBeenCalled()
  })
  it('pans without editing and reports pixel coordinates and font scale', () => {
    const { canvas, onEdit, onCommit } = mount('hand')
    const area = canvas.parentElement!
    area.scrollLeft = 40
    area.scrollTop = 30
    fireEvent.pointerDown(canvas, { button: 0, clientX: 15, clientY: 15 })
    fireEvent.pointerMove(canvas, { clientX: 25, clientY: 20 })
    fireEvent.pointerUp(canvas)
    expect(area.scrollLeft).toBe(30)
    expect(area.scrollTop).toBe(25)
    expect(onEdit).not.toHaveBeenCalled()
    expect(onCommit).not.toHaveBeenCalled()
    expect(screen.getByText('50 font units/pixel')).toBeInTheDocument()
  })
})
