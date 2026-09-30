import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport, type FontEditorSettings } from '../../api'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../test/mockTransport'

import { FontSettingsEditor } from './FontSettingsEditor'

/** A kernel double that resolves stored overrides against fixed preferences. */
function settingsTransport(
  extra: Parameters<typeof createMockTransport>[0] = {},
): MockTransport & { stored: () => FontEditorSettings } {
  let stored: FontEditorSettings = {}
  const transport = createMockTransport({
    'fontinfo.editor_settings_get': () => resolve(stored),
    'fontinfo.editor_settings_set': (params) => {
      stored = structuredClone(params.settings)
      return resolve(stored)
    },
    ...extra,
  })
  return Object.assign(transport, { stored: () => stored })
}

function resolve(stored: FontEditorSettings) {
  return {
    stored: structuredClone(stored),
    grid_size: stored.grid_size ?? 1,
    grid_subdivision: stored.grid_subdivision ?? 1,
    nudge: stored.nudge ?? ([1, 10, 100] as [number, number, number]),
  }
}

beforeEach(() => vi.restoreAllMocks())
afterEach(() => setTransport(undefined))

it('shows the inherited preference until the font overrides it', async () => {
  const transport = settingsTransport()
  setTransport(transport)
  const user = userEvent.setup()
  render(<FontSettingsEditor doc={1} unitsPerEm={1000} />)
  await waitFor(() =>
    expect(
      screen.getAllByText('Inherited from preferences: 1', { exact: false }),
    ).toHaveLength(3),
  )
  expect(screen.getByLabelText('Grid size (font units)')).toBeDisabled()

  await user.click(screen.getByLabelText('Set a grid size for this font'))
  await waitFor(() => expect(transport.stored().grid_size).toBe(1))
  expect(screen.getByLabelText('Grid size (font units)')).toBeEnabled()
  await waitFor(() =>
    expect(
      screen.getByText('Stored in this font: 1', { exact: false }),
    ).toBeInTheDocument(),
  )

  // Clearing the override returns to the preference.
  await user.click(screen.getByLabelText('Set a grid size for this font'))
  await waitFor(() => expect(transport.stored().grid_size).toBeUndefined())
})

it('stores the three keyboard increments together', async () => {
  const transport = settingsTransport()
  setTransport(transport)
  const user = userEvent.setup()
  render(<FontSettingsEditor doc={1} unitsPerEm={1000} />)
  await waitFor(() =>
    expect(
      screen.getByLabelText('Set keyboard increments for this font'),
    ).toBeEnabled(),
  )
  await user.click(
    screen.getByLabelText('Set keyboard increments for this font'),
  )
  await waitFor(() => expect(transport.stored().nudge).toEqual([1, 10, 100]))
  const shift = screen.getByLabelText('Shift + arrow')
  await user.clear(shift)
  await user.type(shift, '20')
  await user.tab()
  await waitFor(() => expect(transport.stored().nudge).toEqual([1, 20, 100]))
})

it('reports a rejected setting instead of showing it as stored', async () => {
  setTransport(
    settingsTransport({
      'fontinfo.editor_settings_set': () => {
        throw new Error('grid subdivision must be between 1 and 100')
      },
    }),
  )
  const user = userEvent.setup()
  render(<FontSettingsEditor doc={1} unitsPerEm={1000} />)
  await waitFor(() =>
    expect(
      screen.getByLabelText('Set a subdivision for this font'),
    ).toBeEnabled(),
  )
  await user.click(screen.getByLabelText('Set a subdivision for this font'))
  await waitFor(() =>
    expect(screen.getByRole('alert')).toHaveTextContent('between 1 and 100'),
  )
})

it('changes units per em with the chosen scaling groups and reports the result', async () => {
  const transport = settingsTransport({
    'variation.state': () => ({
      generation: 12,
      axes: [],
      masters: [],
      instances: [],
      first_glyph: null,
    }),
    'variation.rescale_upm': () => ({
      factor: 2,
      glyphs: 3,
      layers: 4,
      cleared_programs: 2,
      warnings: [
        'Preserved imported tables still describe the previous units per em.',
      ],
      change: noChange(),
    }),
  })
  setTransport(transport)
  const user = userEvent.setup()
  render(<FontSettingsEditor doc={1} unitsPerEm={1000} />)
  await waitFor(() =>
    expect(screen.getByLabelText('Units per em')).toBeInTheDocument(),
  )
  expect(
    screen.getByRole('button', { name: 'Change units per em' }),
  ).toBeDisabled()
  const upm = screen.getByLabelText('Units per em')
  await user.clear(upm)
  await user.type(upm, '2000')
  await user.tab()
  await user.click(screen.getByLabelText('Scale kerning'))
  await user.click(screen.getByRole('button', { name: 'Change units per em' }))
  await waitFor(() =>
    expect(
      transport.calls.find((call) => call.method === 'variation.rescale_upm')
        ?.params,
    ).toMatchObject({
      doc: 1,
      expected_generation: 12,
      units_per_em: 2000,
      scale_outlines: true,
      scale_metrics: true,
      scale_kerning: false,
      scale_guides: true,
    }),
  )
  const notices = await screen.findAllByRole('status')
  const text = notices.map((node) => node.textContent ?? '').join(' ')
  expect(text).toContain('3 glyphs')
  expect(text).toContain('2 layers')
  expect(text).toContain('previous units per em')
})

it('reports a refused units-per-em change without claiming it happened', async () => {
  setTransport(
    settingsTransport({
      'variation.state': () => ({
        generation: 1,
        axes: [],
        masters: [],
        instances: [],
        first_glyph: null,
      }),
      'variation.rescale_upm': () => {
        throw new Error('A carries layer effects')
      },
    }),
  )
  const user = userEvent.setup()
  render(<FontSettingsEditor doc={1} unitsPerEm={1000} />)
  const upm = await screen.findByLabelText('Units per em')
  await user.clear(upm)
  await user.type(upm, '2048')
  await user.tab()
  await user.click(screen.getByRole('button', { name: 'Change units per em' }))
  await waitFor(() =>
    expect(screen.getByRole('alert')).toHaveTextContent('layer effects'),
  )
  expect(screen.queryAllByRole('status')).toHaveLength(0)
})
