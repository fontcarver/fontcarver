import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import {
  setTransport,
  type BitmapGlyph,
  type BitmapStrikeInfo,
} from '../../api'
import { decodeSheetPng } from '../../canvas/bitmapSheetImage'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { BitmapSheet } from './BitmapSheet'

vi.mock('../../canvas/bitmapSheetImage', () => ({
  decodeSheetPng: vi.fn(),
  encodeSheetPng: vi.fn(),
}))
afterEach(() => {
  cleanup()
  setTransport(undefined)
  vi.clearAllMocks()
})

const strike: BitmapStrikeInfo = {
  ppem: 2,
  depth: 8,
  ascent: 1,
  descent: 1,
  properties: {},
  glyph_count: 1,
}
const glyph: BitmapGlyph = {
  bitmap: {
    width: 1,
    height: 1,
    origin_x: -1,
    origin_y: -1,
    depth: 8,
    rows: [80],
  },
  advance: 3,
  vertical_advance: 4,
  refs: [],
}

function setup(stale = false) {
  const apply = vi.fn(() => noChange())
  const mock = createMockTransport({
    'bitmap.state': () => ({ generation: 5, strikes: [strike] }),
    'bitmap.glyph': () => ({
      generation: stale ? 6 : 5,
      value: glyph,
      resolved: glyph,
    }),
    'bitmap.apply': apply,
  })
  setTransport(mock)
  const data = new Uint8ClampedArray(16)
  data[11] = 255 // Cell's bottom-left pixel.
  vi.mocked(decodeSheetPng).mockResolvedValue({ width: 2, height: 2, data })
  render(<BitmapSheet doc={904} strike={strike} />)
  fireEvent.click(screen.getByText('PNG sprite sheet'))
  fireEvent.change(screen.getByLabelText('Glyph names in cell order'), {
    target: { value: 'A' },
  })
  fireEvent.change(screen.getByLabelText('Columns'), { target: { value: '1' } })
  fireEvent.change(screen.getByLabelText('PNG file'), {
    target: { files: [new File(['png'], 'sheet.png', { type: 'image/png' })] },
  })
  return apply
}

it('imports all selected images with one generation-checked undo command', async () => {
  const apply = setup()
  fireEvent.click(
    screen.getByRole('button', { name: 'Import PNG into strike' }),
  )
  await screen.findByText('Imported 1 glyph. Undo restores its previous image.')
  expect(apply).toHaveBeenCalledExactlyOnceWith({
    doc: 904,
    expected_generation: 5,
    label: 'Import bitmap sprite sheet',
    command: {
      SetGlyphs: {
        ppem: 2,
        depth: 8,
        glyphs: { A: { ...glyph, bitmap: { ...glyph.bitmap, rows: [255] } } },
      },
    },
  })
})

it('refuses a stale glyph snapshot before submitting any write', async () => {
  const apply = setup(true)
  fireEvent.click(
    screen.getByRole('button', { name: 'Import PNG into strike' }),
  )
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The strike changed while processing.',
  )
  expect(apply).not.toHaveBeenCalled()
  expect(screen.getByLabelText('Glyph names in cell order')).toHaveValue('A')
})

it('keeps import errors in the panel without changing the document', async () => {
  const apply = setup()
  vi.mocked(decodeSheetPng).mockRejectedValue(new Error('invalid PNG'))
  fireEvent.click(
    screen.getByRole('button', { name: 'Import PNG into strike' }),
  )
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The sprite sheet could not be processed.',
  )
  expect(apply).not.toHaveBeenCalled()
})
