import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type BitmapStrikeInfo } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { BitmapTransfer } from './BitmapTransfer'
import { BitmapOutlineTools } from './BitmapOutlineTools'
import { cropGlyph, resample } from './bitmapTransforms'
import { blank, put } from './pixels'

const strike: BitmapStrikeInfo = {
  ppem: 10,
  depth: 1,
  ascent: 8,
  descent: 2,
  properties: {},
  glyph_count: 2,
}
afterEach(() => setTransport(undefined))

describe('bitmap conversion controls', () => {
  it('copies the selected glyph with fresh generation and target scale options', async () => {
    const transfer = vi.fn(() => noChange())
    setTransport(
      createMockTransport({
        'bitmap.state': () => ({ generation: 7, strikes: [strike] }),
        'bitmap.transfer': transfer,
      }),
    )
    render(<BitmapTransfer doc={801} strike={strike} glyph="A" />)
    fireEvent.click(screen.getByText('Copy between strikes'))
    fireEvent.change(screen.getByLabelText('Target depth'), {
      target: { value: '8' },
    })
    fireEvent.click(
      screen.getByRole('button', { name: 'Copy to target strike' }),
    )
    await waitFor(() =>
      expect(transfer).toHaveBeenCalledWith(
        expect.objectContaining({
          expected_generation: 7,
          source_ppem: 10,
          source_depth: 1,
          target_ppem: 20,
          target_depth: 8,
          glyphs: ['A'],
          scale: true,
        }),
      ),
    )
  })
  it('selects round tracing and diagonal joins independently', async () => {
    const trace = vi.fn(() => noChange())
    setTransport(
      createMockTransport({
        'bitmap.state': () => ({ generation: 8, strikes: [strike] }),
        'bitmap.trace': trace,
      }),
    )
    render(
      <BitmapOutlineTools
        doc={802}
        strike={strike}
        glyph="A"
        master="master"
      />,
    )
    fireEvent.change(screen.getByLabelText('Pixel corner mode'), {
      target: { value: 'RoundedCorners' },
    })
    fireEvent.click(screen.getByLabelText('Join diagonal pixels'))
    fireEvent.click(
      screen.getByRole('button', { name: 'Replace outline with pixel trace' }),
    )
    await waitFor(() =>
      expect(trace).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: 'RoundedCorners',
          join_diagonals: true,
          threshold: 128,
          expected_generation: 8,
        }),
      ),
    )
  })
  it('constructs the whole strike using the named persistent pixel glyph', async () => {
    const components = vi.fn(() => noChange())
    setTransport(
      createMockTransport({
        'bitmap.state': () => ({ generation: 9, strikes: [strike] }),
        'bitmap.pixel_components': components,
      }),
    )
    render(
      <BitmapOutlineTools
        doc={803}
        strike={strike}
        glyph="A"
        master="master"
      />,
    )
    fireEvent.click(screen.getByText('Editable pixel components'))
    fireEvent.change(screen.getByLabelText('Pixel glyph name'), {
      target: { value: 'dot' },
    })
    fireEvent.click(screen.getByLabelText('All images in this strike'))
    fireEvent.click(
      screen.getByRole('button', { name: 'Build pixel components' }),
    )
    await waitFor(() =>
      expect(components).toHaveBeenCalledWith(
        expect.objectContaining({
          pixel_glyph: 'dot',
          glyphs: [],
          expected_generation: 9,
        }),
      ),
    )
  })
  it('crops and resamples local pixels without destroying linked bitmap references', () => {
    const bitmap = blank(4, 4, 1)
    put(bitmap, 1, 1, 1)
    const source = {
      bitmap,
      advance: 4,
      vertical_advance: null,
      refs: [{ base: 'accent', dx: 2, dy: 3 }],
    }
    const cropped = cropGlyph(source)
    expect(cropped.refs).toEqual(source.refs)
    expect(cropped.bitmap.width).toBe(1)
    expect(resample(source, 8, 8, false).refs).toEqual(source.refs)
  })
})
