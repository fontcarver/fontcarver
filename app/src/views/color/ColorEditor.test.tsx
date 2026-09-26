import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type ColorState, type ColorTable } from '../../api'
import {
  flushDocumentEditParticipants,
  suspendDocumentEditParticipants,
} from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { ColorEditor } from './ColorEditor'
import { emptyTable, parseColor, removeEntry } from './colorModel'

afterEach(() => setTransport(undefined))

function setup(color: ColorTable = emptyTable()) {
  let state: ColorState = {
    generation: 2,
    status: 'Editable',
    color,
    import_error: null,
    units_per_em: 1000,
  }
  const mock = createMockTransport({
    'color.state': () => state,
    'color.apply': ({ color, expected_generation }) => {
      if (expected_generation !== state.generation)
        throw {
          code: 'invalid_params',
          message: 'Color data changed; reload the editor before applying',
        }
      state = { ...state, color, generation: state.generation + 1 }
      return { ...noChange(), color: true }
    },
    'color.preview': ({ glyph, layers, palette }) => ({
      generation: state.generation,
      width: 600,
      units_per_em: 1000,
      layers: (layers ?? state.color?.glyphs[glyph] ?? []).map((layer) => ({
        glyph: layer.glyph,
        path:
          layer.glyph === 'A' ? 'M0 0L100 0L100 100Z' : 'M50 0L150 0L150 100Z',
        bounds: [0, 0, 150, 100],
        color:
          layer.palette_index === null
            ? null
            : (palette ?? state.color!.palettes[0].colors)[layer.palette_index],
      })),
    }),
    'glyphs.list': () => ({ total: 0, items: [] }),
  })
  setTransport(mock)
  const close = vi.fn()
  render(<ColorEditor doc={902} onClose={close} />)
  return {
    mock,
    close,
    state: () => state,
    changeRevision: () => {
      state = { ...state, generation: state.generation + 1 }
    },
    user: userEvent.setup(),
  }
}

it('applies an RGBA palette edit and retains selection', async () => {
  const { user, state, mock } = setup()
  const input = await screen.findByRole('textbox', { name: 'Entry 0 RGBA' })
  fireEvent.change(input, { target: { value: '#FF200080' } })
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Apply changes' }),
    ).toBeDisabled(),
  )
  expect(state().color?.palettes[0].colors[0]).toEqual({
    red: 255,
    green: 32,
    blue: 0,
    alpha: 128,
  })
  expect(mock.calls.filter((c) => c.method === 'color.apply')).toHaveLength(1)
})

it('retains invalid text on close and suspended save, with explicit discard', async () => {
  const { user, close, mock } = setup()
  const input = await screen.findByRole('textbox', { name: 'Entry 0 RGBA' })
  fireEvent.change(input, { target: { value: '#BAD' } })
  await user.click(screen.getByRole('button', { name: 'Close' }))
  expect(close).not.toHaveBeenCalled()
  let resume = () => {}
  act(() => {
    resume = suspendDocumentEditParticipants(902)
  })
  try {
    await act(async () => {
      await expect(flushDocumentEditParticipants(902)).rejects.toThrow()
    })
  } finally {
    act(resume)
  }
  expect(input).toHaveValue('#BAD')
  expect(mock.calls.filter((c) => c.method === 'color.apply')).toHaveLength(0)
  await user.click(screen.getByRole('button', { name: 'Discard and close' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
})

it('keeps palettes aligned when adding entries and duplicating a palette', async () => {
  const { user, state } = setup()
  await screen.findByRole('textbox', { name: 'Entry 0 RGBA' })
  await user.click(screen.getByRole('button', { name: 'Duplicate palette' }))
  await user.click(
    screen.getByRole('button', { name: 'Add entry to all palettes' }),
  )
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(state().color?.palettes).toHaveLength(2))
  expect(state().color?.palettes.map((p) => p.colors.length)).toEqual([2, 2])
})

it('reindexes unused entries without changing foreground or used entries', () => {
  const table = emptyTable()
  table.palettes[0].colors.push({ red: 1, green: 2, blue: 3, alpha: 4 })
  table.glyphs.A = [
    { glyph: 'A', palette_index: 1 },
    { glyph: 'B', palette_index: null },
  ]
  expect(removeEntry(table, 1)).toBe(table)
  expect(removeEntry(table, 0).glyphs.A.map((l) => l.palette_index)).toEqual([
    0,
    null,
  ])
  expect(parseColor('#12aBcD')).toEqual({
    red: 18,
    green: 171,
    blue: 205,
    alpha: 255,
  })
  expect(parseColor('#12abcg')).toBeUndefined()
})

it('reorders outline layers and previews alpha and foreground in paint order', async () => {
  const table = emptyTable()
  table.palettes[0].colors[0].alpha = 128
  table.glyphs.A = [
    { glyph: 'A', palette_index: 0 },
    { glyph: 'B', palette_index: null },
  ]
  const { user, state } = setup(table)
  await screen.findByLabelText('Layer 1 outline glyph')
  const image = screen.getByRole('img', { name: 'Color preview' })
  await waitFor(() => expect(image.querySelectorAll('path')).toHaveLength(2))
  expect(image.querySelectorAll('path')[0]).toHaveAttribute(
    'fill-opacity',
    String(128 / 255),
  )
  expect(image.querySelectorAll('path')[1]).toHaveAttribute('fill', '#202020')
  await user.click(screen.getByRole('button', { name: 'Move layer 2 down' }))
  expect(screen.getByLabelText('Layer 1 outline glyph')).toHaveValue('B')
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Layer 1 color' }),
    '0',
  )
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() =>
    expect(state().color?.glyphs.A[0]).toEqual({
      glyph: 'B',
      palette_index: 0,
    }),
  )
})

it('does not switch palettes or glyphs over invalid RGBA text', async () => {
  const table = emptyTable()
  table.palettes.push(structuredClone(table.palettes[0]))
  table.glyphs.A = [{ glyph: 'A', palette_index: 0 }]
  table.glyphs.B = [{ glyph: 'B', palette_index: null }]
  const { user } = setup(table)
  const input = await screen.findByRole('textbox', { name: 'Entry 0 RGBA' })
  fireEvent.change(input, { target: { value: '#X' } })
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Choose palette' }),
    '1',
  )
  expect(screen.getByRole('combobox', { name: 'Choose palette' })).toHaveValue(
    '0',
  )
  await user.selectOptions(
    screen.getByRole('listbox', { name: 'Choose layered glyph' }),
    'B',
  )
  expect(screen.getByLabelText('Layer 1 outline glyph')).toHaveValue('A')
  expect(input).toHaveValue('#X')
})

it('keeps a rejected stale draft visible until explicit reload', async () => {
  const { user, changeRevision, close, state } = setup()
  const input = await screen.findByRole('textbox', { name: 'Entry 0 RGBA' })
  fireEvent.change(input, { target: { value: '#FF000077' } })
  changeRevision()
  await user.click(screen.getByRole('button', { name: 'Close' }))
  await screen.findByRole('alert')
  expect(close).not.toHaveBeenCalled()
  expect(input).toHaveValue('#FF000077')
  expect(state().color?.palettes[0].colors[0].alpha).toBe(255)
  await user.click(
    screen.getByRole('button', { name: 'Discard draft and reload' }),
  )
  await waitFor(() => expect(input).toHaveValue('#2A70DCFF'))
})

it('bounds large palette and layer controls while retaining invalid page input', async () => {
  const table = emptyTable()
  table.palettes[0].colors = Array.from({ length: 1000 }, () => ({
    red: 1,
    green: 2,
    blue: 3,
    alpha: 255,
  }))
  table.glyphs.A = Array.from({ length: 40 }, () => ({
    glyph: 'A',
    palette_index: 999,
  }))
  const { user } = setup(table)
  const paint = await screen.findByRole('textbox', { name: 'Layer 1 color' })
  expect(screen.getAllByLabelText(/outline glyph/)).toHaveLength(16)
  expect(document.querySelectorAll('option').length).toBeLessThan(20)
  fireEvent.change(paint, { target: { value: '1000' } })
  expect(paint).toBeInvalid()
  await user.click(screen.getByRole('button', { name: 'Next layers' }))
  expect(paint).toBeInTheDocument()
  fireEvent.change(paint, { target: { value: '500' } })
  await user.click(screen.getByRole('button', { name: 'Next layers' }))
  expect(screen.getByRole('textbox', { name: 'Layer 17 color' })).toHaveValue(
    '999',
  )
  await user.click(screen.getByRole('button', { name: 'Next layers' }))
  expect(screen.getAllByLabelText(/outline glyph/)).toHaveLength(8)
})
