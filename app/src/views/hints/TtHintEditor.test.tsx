import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type TtHintState } from '../../api'
import { makeMaster } from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { TtHintEditor } from './TtHintEditor'
import { i18n } from '../../i18n'

afterEach(async () => {
  setTransport(undefined)
  await i18n.changeLanguage('en')
})

const state = (): TtHintState => ({
  generation: 9,
  layer: 'layer-1',
  fingerprint: 'tt-points-v1:' + 'a'.repeat(64),
  binding_matches: false,
  authored: false,
  program: { bytes: [], assembly: '', error: null },
  fpgm: { bytes: [], assembly: '', error: null },
  prep: { bytes: [], assembly: '', error: null },
  globals: makeMaster().tt,
  path: 'M0 0H80V700H0Z',
  points: [
    { position: { x: 0, y: 0 }, on_curve: true, contour: 0 },
    { position: { x: 80, y: 0 }, on_curve: true, contour: 0 },
  ],
  phantom_start: 2,
  horizontal_phantoms: [0, 300],
  vertical_phantoms: null,
  component_metrics: false,
  units_per_em: 1000,
})

function editor(close = vi.fn()) {
  render(
    <TtHintEditor
      doc={1}
      master="m1"
      glyph="I"
      layer="layer-1"
      onClose={close}
      onApplied={vi.fn()}
    />,
  )
  return close
}

it('previews without applying, clears obsolete previews, and saves the captured point identity', async () => {
  const value = state()
  const transport = createMockTransport({
    'hints.tt_state': () => value,
    'hints.tt_preview': () => ({
      generation: 9,
      sizes: [
        {
          ppem: 12,
          hinted: 'M0 0H1V8Z',
          unhinted: 'M0 0H.96V8.4Z',
          hinted_advance: 4,
          unhinted_advance: 3.6,
        },
      ],
      warnings: [],
    }),
    'hints.tt_apply': () => noChange(),
  })
  setTransport(transport)
  const close = editor()
  const source = await screen.findByRole('textbox', {
    name: 'Glyph program',
  })
  fireEvent.change(source, { target: { value: 'SVTCA[x]' } })
  await userEvent.click(
    screen.getByRole('button', { name: 'Run instruction preview' }),
  )
  await screen.findByRole('region', { name: 'Interpreter preview' })
  expect(transport.calls.some((call) => call.method === 'hints.tt_apply')).toBe(
    false,
  )
  fireEvent.change(source, { target: { value: 'SVTCA[y]' } })
  expect(
    screen.queryByRole('region', { name: 'Interpreter preview' }),
  ).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
  expect(transport.calls).toContainEqual(
    expect.objectContaining({
      method: 'hints.tt_apply',
      params: expect.objectContaining({
        expected_generation: 9,
        draft: expect.objectContaining({
          fingerprint: value.fingerprint,
          program: 'SVTCA[y]',
        }),
      }),
    }),
  )
})

it('keeps incompatible imported instructions as reference and requires explicit review', async () => {
  const transport = createMockTransport({
    'hints.tt_state': () => ({
      ...state(),
      program: { bytes: [0], assembly: 'SVTCA[y]', error: null },
    }),
    'hints.tt_apply': () => noChange(),
  })
  setTransport(transport)
  editor()
  const source = await screen.findByRole('textbox', {
    name: 'Glyph program',
  })
  expect(source).toHaveValue('')
  expect(screen.getByLabelText('Original Glyph program')).toHaveValue(
    'SVTCA[y]',
  )
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  await userEvent.click(
    screen.getByRole('checkbox', {
      name: 'I have reviewed this draft against the current points and font scale',
    }),
  )
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeEnabled()
})

it('retains stale authored source and surfaces interpreter or revision errors without losing the draft', async () => {
  const transport = createMockTransport({
    'hints.tt_state': () => ({
      ...state(),
      authored: true,
      program: { bytes: [0], assembly: 'SVTCA[y]', error: null },
    }),
    'hints.tt_preview': () => {
      throw new Error('Interpreter stack underflow')
    },
    'hints.tt_apply': () => {
      throw new Error('Revision changed')
    },
  })
  setTransport(transport)
  const close = editor()
  const source = await screen.findByRole('textbox', {
    name: 'Glyph program',
  })
  expect(source).toHaveValue('SVTCA[y]')
  await userEvent.click(
    screen.getByRole('checkbox', {
      name: 'I have reviewed this draft against the current points and font scale',
    }),
  )
  await userEvent.click(
    screen.getByRole('button', { name: 'Run instruction preview' }),
  )
  await screen.findByRole('alert')
  expect(
    screen.queryByRole('region', { name: 'Interpreter preview' }),
  ).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() =>
    expect(screen.getByRole('alert')).toHaveTextContent('Revision changed'),
  )
  expect(source).toHaveValue('SVTCA[y]')
  expect(close).not.toHaveBeenCalled()
})

it('blocks fractional CVT values and invalid maxima before submitting', async () => {
  const transport = createMockTransport({
    'hints.tt_state': state,
    'hints.tt_apply': () => noChange(),
  })
  setTransport(transport)
  editor()
  const cvt = await screen.findByRole('textbox', {
    name: 'Control values (signed integers)',
  })
  fireEvent.change(cvt, { target: { value: '80.5' } })
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(cvt).toBeInvalid()
  expect(transport.calls.some((call) => call.method === 'hints.tt_apply')).toBe(
    false,
  )
  fireEvent.change(cvt, { target: { value: '80, -20' } })
  fireEvent.change(screen.getByRole('spinbutton', { name: 'Maximum zones' }), {
    target: { value: '3' },
  })
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(transport.calls.some((call) => call.method === 'hints.tt_apply')).toBe(
    false,
  )
})

it('does not silently edit the master when opened from a background layer', async () => {
  setTransport(
    createMockTransport({
      'hints.tt_state': () => ({ ...state(), layer: 'another-layer' }),
    }),
  )
  editor()
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'selected master layer',
  )
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
})

it('keeps unsaved assembly when the interface language changes', async () => {
  const transport = createMockTransport({ 'hints.tt_state': state })
  setTransport(transport)
  editor()
  const source = await screen.findByRole('textbox', { name: 'Glyph program' })
  fireEvent.change(source, { target: { value: 'SVTCA[x]' } })
  await act(async () => {
    await i18n.changeLanguage('zh-CN')
  })
  expect(screen.getByRole('textbox', { name: '字形程序' })).toHaveValue(
    'SVTCA[x]',
  )
  expect(
    transport.calls.filter((call) => call.method === 'hints.tt_state'),
  ).toHaveLength(1)
})
