import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type BitmapPreviewLine } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { BitmapWaterfall } from './BitmapWaterfall'
import { blank, put } from './pixels'

function line(ppem: number): BitmapPreviewLine {
  const bitmap = blank(2, 2, 1)
  put(bitmap, 0, 0, 1)
  return {
    ppem,
    depth: 1,
    ascent: 2,
    descent: 0,
    glyphs: [{ bitmap, advance: 2, vertical_advance: null, refs: [] }],
  }
}

function mount(props: { glyph?: string } = {}) {
  const preview = vi.fn((params: { text: string; glyphs?: string[] }) =>
    params.text || params.glyphs?.length ? [line(8), line(16)] : [],
  )
  setTransport(createMockTransport({ 'bitmap.preview': preview }))
  render(<BitmapWaterfall doc={700} revision={1} {...props} />)
  return { preview }
}

afterEach(() => setTransport(undefined))

describe('the multi-strike waterfall', () => {
  it('renders one row per strike for the sample text', async () => {
    const { preview } = mount()
    await waitFor(() =>
      expect(preview).toHaveBeenCalledWith(
        expect.objectContaining({ text: 'FontCarver 字体点阵 0123456789' }),
      ),
    )
    await waitFor(() => {
      expect(screen.getByText('8px · 1bpp')).toBeInTheDocument()
      expect(screen.getByText('16px · 1bpp')).toBeInTheDocument()
    })
  })

  it('switches to the selected glyph alone and disables the sample field', async () => {
    const { preview } = mount({ glyph: 'uni4E2D' })
    const toggle = screen.getByRole('checkbox', {
      name: 'Show only uni4E2D at every size',
    })
    fireEvent.click(toggle)
    await waitFor(() =>
      expect(preview).toHaveBeenLastCalledWith(
        expect.objectContaining({ text: '', glyphs: ['uni4E2D'] }),
      ),
    )
    expect(screen.getByLabelText('Waterfall sample')).toBeDisabled()
    fireEvent.click(toggle)
    await waitFor(() =>
      expect(preview).toHaveBeenLastCalledWith(
        expect.objectContaining({
          text: 'FontCarver 字体点阵 0123456789',
          glyphs: [],
        }),
      ),
    )
  })

  it('shows an unencoded glyph by name, which text cannot address', async () => {
    const { preview } = mount({ glyph: '.notdef' })
    const toggle = screen.getByRole('checkbox', {
      name: 'Show only .notdef at every size',
    })
    expect(toggle).toBeEnabled()
    fireEvent.click(toggle)
    await waitFor(() =>
      expect(preview).toHaveBeenLastCalledWith(
        expect.objectContaining({ text: '', glyphs: ['.notdef'] }),
      ),
    )
    await waitFor(() =>
      expect(screen.getByText('8px · 1bpp')).toBeInTheDocument(),
    )
  })

  it('offers nothing to show when no glyph is selected', () => {
    mount()
    expect(
      screen.getByRole('checkbox', {
        name: 'Show only the selected glyph at every size',
      }),
    ).toBeDisabled()
  })

  it('switches existing strike pixels to a vertical canvas without another kernel request', async () => {
    const { preview } = mount()
    const canvas = await screen.findByRole('img', {
      name: 'Bitmap proof at 8 px, 1 bpp',
    })
    expect(canvas).toHaveAttribute('width', '18')
    expect(canvas).toHaveAttribute('height', '2')
    const calls = preview.mock.calls.length
    fireEvent.click(screen.getByRole('checkbox', { name: 'Vertical writing' }))
    expect(canvas).toHaveAttribute('width', '8')
    expect(canvas).toHaveAttribute('height', '24')
    expect(preview).toHaveBeenCalledTimes(calls)
    fireEvent.click(screen.getByRole('checkbox', { name: 'Vertical writing' }))
    expect(canvas).toHaveAttribute('width', '18')
    expect(canvas).toHaveAttribute('height', '2')
  })
})
