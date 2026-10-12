import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport, type SvgState } from '../../api'
import { flushDocumentEditParticipants } from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { SvgEditor } from './SvgEditor'
import { toTable } from './svgDraft'

beforeEach(() => {
  vi.stubGlobal(
    'URL',
    class extends URL {
      static createObjectURL() {
        return 'blob:svg-test'
      }
      static revokeObjectURL() {}
    },
  )
})
afterEach(() => {
  // Release preview blob URLs while their test implementation still exists.
  cleanup()
  setTransport(undefined)
  vi.unstubAllGlobals()
})
function setup() {
  let state: SvgState = {
    generation: 2,
    status: 'Editable',
    units_per_em: 1000,
    import_error: null,
    svg: {
      documents: [
        {
          source:
            '<svg xmlns="http://www.w3.org/2000/svg"><path id="art"/></svg>',
          glyphs: { A: 'art' },
        },
      ],
    },
  }
  const mock = createMockTransport({
    'svg.state': () => state,
    'svg.apply': ({ expected_generation, svg }) => {
      if (state.generation !== expected_generation) throw new Error('stale SVG')
      if (svg?.documents.some((d) => !d.source.includes('</svg>')))
        throw new Error('invalid XML')
      state = { ...state, svg, generation: state.generation + 1 }
      return { ...noChange(), color: true }
    },
  })
  setTransport(mock)
  const close = vi.fn()
  render(<SvgEditor doc={904} onClose={close} />)
  return {
    close,
    mock,
    state: () => state,
    user: userEvent.setup(),
    stale: () => {
      state = { ...state, generation: state.generation + 1 }
    },
  }
}
it('flushes named bindings and source atomically when the editor closes', async () => {
  const { close, state, user } = setup()
  const bindings = await screen.findByRole('textbox', {
    name: 'Glyph bindings',
  })
  fireEvent.change(bindings, { target: { value: 'A = art\nB = art' } })
  await user.click(screen.getByRole('button', { name: 'Close' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
  expect(state().svg?.documents[0].glyphs).toEqual({ A: 'art', B: 'art' })
})
it('retains invalid bindings across close and document save; discard is explicit', async () => {
  const { close, state, user } = setup()
  const input = await screen.findByRole('textbox', { name: 'Glyph bindings' })
  fireEvent.change(input, { target: { value: 'A = art\nA = different' } })
  await user.click(screen.getByRole('button', { name: 'Close' }))
  expect(close).not.toHaveBeenCalled()
  expect(input).toHaveValue('A = art\nA = different')
  await act(async () => {
    await expect(flushDocumentEditParticipants(904)).rejects.toThrow()
  })
  expect(state().generation).toBe(2)
  await user.click(screen.getByRole('button', { name: 'Discard and close' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
})
it('retains invalid XML and stale drafts without losing the user source', async () => {
  const { close, user, stale } = setup()
  const input = await screen.findByRole('textbox', { name: 'SVG source' })
  fireEvent.change(input, { target: { value: '<invalid' } })
  await user.click(screen.getByRole('button', { name: 'Close' }))
  expect(close).not.toHaveBeenCalled()
  expect(input).toHaveValue('<invalid')
  fireEvent.change(input, { target: { value: '<svg><g/></svg>' } })
  stale()
  await user.click(screen.getByRole('button', { name: 'Close' }))
  expect(close).not.toHaveBeenCalled()
  expect(input).toHaveValue('<svg><g/></svg>')
})
it('keeps prototype-like glyph names as ordinary own properties', () => {
  const table = toTable({
    documents: [{ source: '', bindings: '__proto__ = art\nconstructor = art' }],
  })
  expect(Object.keys(table!.documents[0].glyphs)).toEqual([
    '__proto__',
    'constructor',
  ])
})
