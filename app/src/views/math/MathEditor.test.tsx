import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type MathState } from '../../api'
import {
  flushDocumentEditParticipants,
  suspendDocumentEditParticipants,
} from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { MathEditor } from './MathEditor'
import { emptyGlyph, emptyTable } from './mathModel'

afterEach(() => setTransport(undefined))

function setup() {
  let state: MathState = {
    generation: 3,
    status: 'Editable',
    math: {
      ...emptyTable(),
      glyphs: Object.fromEntries(
        ['A', 'B'].map((name) => [
          name,
          { ...emptyGlyph(), italic_correction: { value: 10, device: null } },
        ]),
      ),
    },
    import_error: null,
    units_per_em: 1000,
  }
  const mock = createMockTransport({
    'math.state': () => state,
    'math.apply': ({ expected_generation, commands }) => {
      expect(expected_generation).toBe(state.generation)
      let math = structuredClone(state.math)
      for (const command of commands) {
        if ('Replace' in command) math = command.Replace.math
        if ('SetGlobals' in command && math)
          Object.assign(math, command.SetGlobals)
        if ('SetGlyph' in command && math) {
          const { glyph, info } = command.SetGlyph
          if (info) math.glyphs[glyph] = info
          else delete math.glyphs[glyph]
        }
      }
      state = { ...state, math, generation: state.generation + 1 }
      return { ...noChange(), math: true }
    },
    'glyphs.list': () => ({ total: 0, items: [] }),
  })
  setTransport(mock)
  const close = vi.fn()
  render(<MathEditor doc={901} onClose={close} />)
  return { mock, close, state: () => state, user: userEvent.setup() }
}

it('applies constants and a glyph record together and keeps the selection', async () => {
  const { mock, state, user } = setup()
  const axis = await screen.findByRole('textbox', { name: 'Axis height' })
  fireEvent.change(axis, { target: { value: '777' } })
  await user.click(screen.getByRole('tab', { name: 'Glyph records' }))
  await user.selectOptions(screen.getByRole('listbox'), 'A')
  fireEvent.change(screen.getByRole('textbox', { name: 'Italic correction' }), {
    target: { value: '25' },
  })
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Apply changes' }),
    ).toBeDisabled(),
  )
  expect(mock.calls.filter((c) => c.method === 'math.apply')).toHaveLength(1)
  expect(state().math?.constants?.axis_height.value).toBe(777)
  expect(state().math?.glyphs.A?.italic_correction?.value).toBe(25)
  expect(screen.getByRole('listbox')).toHaveValue('A')
  expect(
    screen.getByRole('textbox', { name: 'Italic correction' }),
  ).toHaveValue('25')
})

it('keeps invalid text visible on close and allows explicit discard', async () => {
  const { close, mock, user } = setup()
  const axis = await screen.findByRole('textbox', { name: 'Axis height' })
  fireEvent.change(axis, { target: { value: '-' } })
  expect(axis).toBeInvalid()
  await user.click(screen.getByRole('button', { name: 'Close' }))
  expect(close).not.toHaveBeenCalled()
  expect(axis).toHaveValue('-')
  await user.click(screen.getByRole('button', { name: 'Discard and close' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
  expect(mock.calls.filter((c) => c.method === 'math.apply')).toHaveLength(0)
})

it.each([false, true])(
  'blocks invalid handover (suspended: %s), then saves the corrected value',
  async (suspended) => {
    const { mock, state } = setup()
    const axis = await screen.findByRole('textbox', { name: 'Axis height' })
    fireEvent.change(axis, { target: { value: '777' } })
    fireEvent.change(axis, { target: { value: '777x' } })
    let resume = () => {}
    if (suspended)
      act(() => {
        resume = suspendDocumentEditParticipants(901)
      })
    try {
      await act(async () => {
        await expect(flushDocumentEditParticipants(901)).rejects.toThrow()
      })
    } finally {
      act(resume)
    }
    expect(mock.calls.filter((c) => c.method === 'math.apply')).toHaveLength(0)
    expect(axis).toHaveValue('777x')
    fireEvent.change(axis, { target: { value: '778' } })
    await act(() => flushDocumentEditParticipants(901))
    expect(state().math?.constants?.axis_height.value).toBe(778)
  },
)

it('rejects an unnamed variant during a suspended close handover', async () => {
  const { mock, user } = setup()
  await screen.findByRole('textbox', { name: 'Axis height' })
  await user.click(screen.getByRole('tab', { name: 'Glyph records' }))
  await user.selectOptions(screen.getByRole('listbox'), 'A')
  await user.click(
    screen.getByRole('checkbox', { name: 'Vertical construction' }),
  )
  await user.click(screen.getByRole('button', { name: 'Add variant' }))
  let resume = () => {}
  act(() => {
    resume = suspendDocumentEditParticipants(901)
  })
  try {
    await act(async () => {
      await expect(flushDocumentEditParticipants(901)).rejects.toThrow()
    })
    expect(mock.calls.filter((c) => c.method === 'math.apply')).toHaveLength(0)
  } finally {
    act(resume)
  }
})

it('keeps invalid fields from disappearing when changing sections or glyph records', async () => {
  const { user } = setup()
  const axis = await screen.findByRole('textbox', { name: 'Axis height' })
  fireEvent.change(axis, { target: { value: 'bad' } })
  await user.click(screen.getByRole('tab', { name: 'Glyph records' }))
  expect(screen.getByRole('tab', { name: 'Constants' })).toHaveAttribute(
    'aria-selected',
    'true',
  )
  expect(axis).toHaveValue('bad')
  fireEvent.change(axis, { target: { value: '0' } })
  await user.click(screen.getByRole('tab', { name: 'Glyph records' }))
  await user.selectOptions(screen.getByRole('listbox'), 'A')
  const italic = screen.getByRole('textbox', { name: 'Italic correction' })
  fireEvent.change(italic, { target: { value: 'bad' } })
  await user.selectOptions(screen.getByRole('listbox'), 'B')
  expect(screen.getByRole('listbox')).toHaveValue('A')
  expect(italic).toHaveValue('bad')
  fireEvent.change(italic, { target: { value: '11' } })
  await user.selectOptions(screen.getByRole('listbox'), 'B')
  expect(
    screen.getByRole('textbox', { name: 'Italic correction' }),
  ).toHaveValue('10')
})
