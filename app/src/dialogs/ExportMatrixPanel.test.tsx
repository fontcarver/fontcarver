import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, afterEach, expect, it } from 'vitest'
import { setTransport, type ExportOptions } from '../api'
import { useExportProfileStore } from '../stores/exportProfileStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { ExportMatrixPanel } from './ExportMatrixPanel'

const options: ExportOptions = {
  format: 'Ttf',
  variable: false,
  master: 'm1',
  remove_overlap: true,
  keep_tt_instructions: true,
  preserve_untouched_tables: true,
  production_names: false,
  autohint: false,
  table_subset: null,
  add_extrema: false,
  round_coordinates: false,
  decompose_components: false,
  subroutinize: false,
}
const instance = (name: string) => ({
  name,
  location: { wght: 400 },
  style_name: name,
  family_name: null,
  active: true,
  is_variable_named_instance: true,
  user_data: null,
})
const summary = makeSummary()

beforeEach(() => {
  localStorage.clear()
  useExportProfileStore.getState().reload()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
})
afterEach(() => setTransport(undefined))

it('shows every actual result and continues after an individual export fails', async () => {
  useExportProfileStore.getState().save('Web', options)
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: [instance('Regular'), instance('Bold')],
      first_glyph: null,
    }),
    'session.export': ({ path, options: item }) => {
      if (item.instance === 0) throw new Error('destination is read-only')
      return {
        path,
        format: item.format,
        bytes_written: 123,
        warnings: [],
        tables: [],
        touched_glyphs: 2,
        timing_ms: 3,
      }
    },
  })
  setTransport(transport)
  const user = userEvent.setup()
  render(
    <ExportMatrixPanel
      summary={summary}
      destination="/exports/font.ttf"
      busy={false}
      onBusyChange={() => undefined}
    />,
  )
  await user.click(screen.getByText('Batch export'))
  await user.click(screen.getByRole('button', { name: 'Load named instances' }))
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Bold' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Export batch' }))
  const results = await screen.findByRole('list', { name: 'Batch results' })
  await waitFor(() =>
    expect(within(results).getAllByRole('listitem')).toHaveLength(2),
  )
  expect(results).toHaveTextContent('destination is read-only')
  expect(results).toHaveTextContent('/exports/Bold-Web.ttf')
  expect(results).toHaveTextContent('123')
  const exports = transport.calls.filter(
    (call) => call.method === 'session.export',
  )
  expect(exports).toHaveLength(2)
  expect(exports[0]?.params).toMatchObject({
    options: {
      instance: 0,
      variable: false,
      master: null,
      keep_tt_instructions: false,
    },
  })
  expect(exports[1]?.params).toMatchObject({ options: { instance: 1 } })
})

it('blocks the entire batch when different selections generate the same path', async () => {
  useExportProfileStore.getState().save('Web', options)
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: [instance('Regular'), instance('regular')],
      first_glyph: null,
    }),
  })
  setTransport(transport)
  const user = userEvent.setup()
  render(
    <ExportMatrixPanel
      summary={summary}
      destination="/exports/font.ttf"
      busy={false}
      onBusyChange={() => undefined}
    />,
  )
  await user.click(screen.getByText('Batch export'))
  await user.click(screen.getByRole('button', { name: 'Load named instances' }))
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  expect(screen.getByRole('alert')).toHaveTextContent('same destination')
  expect(screen.getByRole('button', { name: 'Export batch' })).toBeDisabled()
  expect(transport.calls.some((call) => call.method === 'session.export')).toBe(
    false,
  )
})

it('requires a refreshed instance list if the design space changed before export', async () => {
  useExportProfileStore.getState().save('Web', options)
  let generation = 0
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: generation++,
      axes: [],
      masters: [],
      instances: [instance('Regular')],
      first_glyph: null,
    }),
  })
  setTransport(transport)
  const user = userEvent.setup()
  render(
    <ExportMatrixPanel
      summary={summary}
      destination="/exports/font.ttf"
      busy={false}
      onBusyChange={() => undefined}
    />,
  )
  await user.click(screen.getByText('Batch export'))
  await user.click(screen.getByRole('button', { name: 'Load named instances' }))
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Export batch' }))
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The design space changed',
  )
  expect(transport.calls.some((call) => call.method === 'session.export')).toBe(
    false,
  )
})
