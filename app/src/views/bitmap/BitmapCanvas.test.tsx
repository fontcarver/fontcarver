import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { BitmapCanvas } from './BitmapCanvas'
import { useBitmapToolStore, readOverlays } from './bitmapToolStore'
import {
  blank,
  put,
  sample,
  type PixelSelection,
  type PixelTool,
} from './pixels'
import type { BitmapGlyph } from '../../api'

vi.mock('../../canvas/bitmapRenderer', () => ({ renderBitmap: vi.fn() }))
const glyph = (): BitmapGlyph => ({
  bitmap: blank(3, 3, 1),
  advance: 4,
  vertical_advance: null,
  refs: [],
})
function mount(tool: PixelTool, value = glyph(), selection?: PixelSelection) {
  const onEdit = vi.fn(),
    onCommit = vi.fn(),
    onPickInk = vi.fn(),
    onSelect = vi.fn()
  render(
    <BitmapCanvas
      value={value}
      zoom={10}
      tool={tool}
      ink={1}
      disabled={false}
      selection={selection}
      onSelect={onSelect}
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
  return { canvas, onEdit, onCommit, onPickInk, onSelect }
}

beforeEach(() => {
  vi.stubGlobal('PointerEvent', MouseEvent)
  localStorage.clear()
  useBitmapToolStore.setState({ overlays: readOverlays() })
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

describe('marquee move and copy', () => {
  const selection: PixelSelection = { x: 0, y: 0, width: 1, height: 1 }
  it('moves the selected pixels by default', () => {
    const value = glyph()
    put(value.bitmap, 0, 0, 1)
    const { canvas, onEdit } = mount('select', value, selection)
    fireEvent.pointerDown(canvas, { button: 0, clientX: 15, clientY: 15 })
    fireEvent.pointerMove(canvas, { clientX: 35, clientY: 15 })
    fireEvent.pointerUp(canvas)
    const edited = onEdit.mock.lastCall?.[0] as BitmapGlyph
    expect([0, 1, 2].map((x) => sample(edited.bitmap, x, 0))).toEqual([0, 0, 1])
  })
  it('stamps a copy while Alt is held, leaving the original pixels', () => {
    const value = glyph()
    put(value.bitmap, 0, 0, 1)
    const { canvas, onEdit, onCommit } = mount('select', value, selection)
    fireEvent.pointerDown(canvas, {
      button: 0,
      clientX: 15,
      clientY: 15,
      altKey: true,
    })
    fireEvent.pointerMove(canvas, { clientX: 35, clientY: 15 })
    fireEvent.pointerUp(canvas)
    const edited = onEdit.mock.lastCall?.[0] as BitmapGlyph
    expect([0, 1, 2].map((x) => sample(edited.bitmap, x, 0))).toEqual([1, 0, 1])
    expect(onCommit).toHaveBeenCalledTimes(1)
  })
})

describe('overlay switches and the font-unit readout', () => {
  it('reports the pointer and the drag delta in pixels and font units', () => {
    const { canvas } = mount('pencil')
    fireEvent.pointerDown(canvas, { button: 0, clientX: 15, clientY: 15 })
    fireEvent.pointerMove(canvas, { clientX: 35, clientY: 15 })
    const readout = screen.getByLabelText('Bitmap pointer coordinates')
    // 50 font units per pixel at 20 ppem on a 1000-unit em.
    expect(readout.textContent).toContain('X 2, Y 2 px')
    expect(readout.textContent).toContain('(100, 100 units)')
    expect(readout.textContent).toContain('ΔX 2, ΔY 0 px')
    expect(readout.textContent).toContain('(100, 0 units)')
    fireEvent.pointerUp(canvas)
  })
  it('remembers every overlay switch across remounts', () => {
    const { unmount } = render(<div />)
    unmount()
    mount('pencil')
    for (const label of ['Pixel grid', 'Advance column', 'Font units']) {
      const box = screen.getByRole('checkbox', { name: label })
      expect(box).toBeChecked()
      fireEvent.click(box)
      expect(box).not.toBeChecked()
    }
    expect(
      JSON.parse(localStorage.getItem('fontcarver.bitmapView.v1')!),
    ).toEqual({
      grid: false,
      outline: false,
      advanceColumn: false,
      fontUnits: false,
    })
    expect(readOverlays()).toEqual({
      grid: false,
      outline: false,
      advanceColumn: false,
      fontUnits: false,
    })
  })
  it('hides the unit readouts when font units are switched off', () => {
    mount('pencil')
    fireEvent.click(screen.getByRole('checkbox', { name: 'Font units' }))
    const canvas = screen.getByLabelText('Bitmap pixel canvas')
    fireEvent.pointerDown(canvas, { button: 0, clientX: 15, clientY: 15 })
    const readout = screen.getByLabelText('Bitmap pointer coordinates')
    expect(readout.textContent).toContain('X 0, Y 2 px')
    expect(readout.textContent).not.toContain('units)')
    // The standing units-per-pixel readout is not one of the HUD values.
    expect(screen.getByText('50 font units/pixel')).toBeInTheDocument()
    fireEvent.pointerUp(canvas)
  })
})
