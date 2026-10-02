import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import {
  setTransport,
  type BitmapGlyph,
  type BitmapStrikeInfo,
} from '../../api'
import { decodeSheetPng, encodeSheetPng } from '../../canvas/bitmapSheetImage'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { BitmapSheet } from './BitmapSheet'

vi.mock('../../canvas/bitmapSheetImage', () => ({
  decodeSheetPng: vi.fn(),
  encodeSheetPng: vi.fn(),
}))
const native = vi.hoisted(() => ({ save: vi.fn(), writeFile: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: native.save }))
vi.mock('@tauri-apps/plugin-fs', () => ({ writeFile: native.writeFile }))
afterEach(() => {
  cleanup()
  setTransport(undefined)
  vi.clearAllMocks()
  vi.unstubAllGlobals()
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

function setup(stale = false, tauri = false) {
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
  setTransport(tauri ? { ...mock, kind: 'tauri' } : mock)
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

async function prepareNativePng() {
  vi.stubGlobal('URL', {
    createObjectURL: vi.fn(() => 'blob:sheet'),
    revokeObjectURL: vi.fn(),
  })
  const bytes = new Uint8Array([137, 80, 78, 71])
  const blob = new Blob([bytes], { type: 'image/png' })
  blob.arrayBuffer = async () => bytes.buffer
  vi.mocked(encodeSheetPng).mockResolvedValue(blob)
  const apply = setup(false, true)
  fireEvent.click(screen.getByRole('button', { name: 'Prepare PNG export' }))
  await screen.findByAltText('Exported bitmap sprite sheet')
  return {
    apply,
    bytes,
    save: screen.getByRole('button', { name: 'Save PNG…' }),
  }
}

it('saves prepared PNG bytes through the native dialog and scoped filesystem', async () => {
  native.save.mockResolvedValue('/chosen/sheet.png')
  native.writeFile.mockResolvedValue(undefined)
  const { apply, bytes, save } = await prepareNativePng()
  fireEvent.click(save)
  await screen.findByText('PNG saved.')
  expect(native.save).toHaveBeenCalledExactlyOnceWith({
    defaultPath: 'bitmap-2px-8bpp.png',
    filters: [{ name: 'PNG sprite sheet', extensions: ['png'] }],
  })
  expect(native.writeFile).toHaveBeenCalledExactlyOnceWith(
    '/chosen/sheet.png',
    bytes,
  )
  expect(apply).not.toHaveBeenCalled()
})

it('keeps the prepared preview after native cancellation without writing', async () => {
  native.save.mockResolvedValue(null)
  const { save } = await prepareNativePng()
  fireEvent.click(save)
  await waitFor(() => expect(native.save).toHaveBeenCalledOnce())
  await waitFor(() => expect(save).toBeEnabled())
  expect(native.writeFile).not.toHaveBeenCalled()
  expect(
    screen.getByAltText('Exported bitmap sprite sheet'),
  ).toBeInTheDocument()
  expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  expect(screen.queryByText('PNG saved.')).not.toBeInTheDocument()
})

it('retains the PNG after a native write failure and permits retry', async () => {
  native.save.mockResolvedValue('/chosen/sheet.png')
  native.writeFile
    .mockRejectedValueOnce(new Error('disk full'))
    .mockResolvedValue(undefined)
  const { save } = await prepareNativePng()
  fireEvent.click(save)
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The PNG could not be saved. Choose another location or retry.',
  )
  expect(
    screen.getByAltText('Exported bitmap sprite sheet'),
  ).toBeInTheDocument()
  fireEvent.click(save)
  await screen.findByText('PNG saved.')
  expect(native.writeFile).toHaveBeenCalledTimes(2)
  expect(screen.queryByRole('alert')).not.toBeInTheDocument()
})
