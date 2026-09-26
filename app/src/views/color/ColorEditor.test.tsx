import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type ColorState } from '../../api'
import {
  flushDocumentEditParticipants,
  suspendDocumentEditParticipants,
} from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { ColorEditor } from './ColorEditor'
import { emptyTable, parseColor, removeEntry } from './colorModel'

afterEach(() => setTransport(undefined))

function setup() {
  let state: ColorState = {
    generation: 2,
    status: 'Editable',
    color: emptyTable(),
    import_error: null,
    units_per_em: 1000,
  }
  const mock = createMockTransport({
    'color.state': () => state,
    'color.apply': ({ color, expected_generation }) => {
      expect(expected_generation).toBe(state.generation)
      state = { ...state, color, generation: state.generation + 1 }
      return { ...noChange(), color: true }
    },
    'glyphs.list': () => ({ total: 0, items: [] }),
  })
  setTransport(mock)
  const close = vi.fn()
  render(<ColorEditor doc={902} onClose={close} />)
  return { mock, close, state: () => state, user: userEvent.setup() }
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
