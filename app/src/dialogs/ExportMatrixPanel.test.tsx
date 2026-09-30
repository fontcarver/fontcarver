import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, afterEach, expect, it } from 'vitest'
import {
  setTransport,
  type ExportOptions,
  type ExportProfile,
  type VariableExportConfig,
} from '../api'
import { useExportProfileStore } from '../stores/exportProfileStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { ExportMatrixPanel } from './ExportMatrixPanel'

const options: ExportOptions = {
  format: 'Ttf',
  variable: false,
  master: null,
  remove_overlap: true,
  keep_tt_instructions: false,
  preserve_untouched_tables: true,
  production_names: false,
  autohint: false,
  table_subset: null,
  add_extrema: false,
  round_coordinates: false,
  decompose_components: false,
  subroutinize: false,
}
const profile: ExportProfile = {
  name: 'Web',
  subfolder: '',
  file_name: '',
  options,
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

/** What the kernel resolver returns for one cell. */
function resolved(params: {
  profile: string
  instance?: string | null
  variable_config?: string | null
}) {
  const row = params.instance ?? params.variable_config ?? 'Default'
  return {
    options: {
      ...options,
      ...(params.instance
        ? { instance: params.instance === 'Bold' ? 1 : 0 }
        : {}),
      ...(params.variable_config ? { variable: true } : {}),
    },
    relative_path: `${row}-${params.profile}.ttf`,
  }
}

function stored(
  profiles: ExportProfile[],
  variableConfigs: VariableExportConfig[] = [],
) {
  useExportProfileStore.setState({
    docs: { [summary.doc]: { profiles, variableConfigs } },
    loading: {},
    error: null,
    detail: null,
  })
}

beforeEach(() => {
  useExportProfileStore.setState({
    docs: {},
    loading: {},
    error: null,
    detail: null,
  })
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
})
afterEach(() => setTransport(undefined))

it('shows every actual result and continues after an individual export fails', async () => {
  const transport = createMockTransport({
    'export.profiles_get': () => ({
      profiles: [profile],
      variable_configs: [],
    }),
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: [instance('Regular'), instance('Bold')],
      first_glyph: null,
    }),
    'export.resolve_profile': (params) => resolved(params),
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
  await user.click(
    screen.getByRole('button', { name: 'Load instances and profiles' }),
  )
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Bold' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Plan destinations' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Export batch' })).toBeEnabled(),
  )
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
    path: '/exports/Regular-Web.ttf',
    options: { instance: 0, variable: false },
  })
  expect(exports[1]?.params).toMatchObject({ options: { instance: 1 } })
})

it('offers a stored variable-font configuration as its own row', async () => {
  const config: VariableExportConfig = {
    name: 'Text VF',
    file_name: '',
    masters: [],
    axes: ['wght'],
    instances: [],
  }
  const transport = createMockTransport({
    'export.profiles_get': () => ({
      profiles: [profile],
      variable_configs: [config],
    }),
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: [instance('Regular')],
      first_glyph: null,
    }),
    'export.resolve_profile': (params) => resolved(params),
    'session.export': ({ path, options: item }) => ({
      path,
      format: item.format,
      bytes_written: 9,
      warnings: [],
      tables: [],
      touched_glyphs: 0,
      timing_ms: 1,
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
  await user.click(
    screen.getByRole('button', { name: 'Load instances and profiles' }),
  )
  await user.click(
    await screen.findByRole('checkbox', { name: 'Text VF (variable font)' }),
  )
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Plan destinations' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Export batch' })).toBeEnabled(),
  )
  const request = transport.calls.find(
    (call) => call.method === 'export.resolve_profile',
  )
  expect(request?.params).toMatchObject({
    profile: 'Web',
    variable_config: 'Text VF',
  })
  await user.click(screen.getByRole('button', { name: 'Export batch' }))
  const results = await screen.findByRole('list', { name: 'Batch results' })
  await waitFor(() =>
    expect(within(results).getAllByRole('listitem')).toHaveLength(1),
  )
  expect(
    transport.calls.find((call) => call.method === 'session.export')?.params,
  ).toMatchObject({
    path: '/exports/Text VF-Web.ttf',
    options: { variable: true },
  })
})

it('blocks the entire batch when different selections generate the same path', async () => {
  const transport = createMockTransport({
    'export.profiles_get': () => ({
      profiles: [profile],
      variable_configs: [],
    }),
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: [instance('Regular'), instance('regular')],
      first_glyph: null,
    }),
    'export.resolve_profile': () => ({
      options,
      relative_path: 'Same-Web.ttf',
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
  await user.click(
    screen.getByRole('button', { name: 'Load instances and profiles' }),
  )
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Plan destinations' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('same destination')
  expect(screen.getByRole('button', { name: 'Export batch' })).toBeDisabled()
  expect(transport.calls.some((call) => call.method === 'session.export')).toBe(
    false,
  )
})

it('reports a rejected resolution instead of planning a guessed destination', async () => {
  stored([profile])
  const transport = createMockTransport({
    'export.profiles_get': () => ({
      profiles: [profile],
      variable_configs: [],
    }),
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: [instance('Regular')],
      first_glyph: null,
    }),
    'export.resolve_profile': () => {
      throw new Error('no named instance `Regular` exists in this document')
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
  await user.click(
    screen.getByRole('button', { name: 'Load instances and profiles' }),
  )
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Plan destinations' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('Regular')
  expect(screen.getByRole('button', { name: 'Export batch' })).toBeDisabled()
})

it('requires a refreshed instance list if the design space changed before export', async () => {
  let generation = 0
  const transport = createMockTransport({
    'export.profiles_get': () => ({
      profiles: [profile],
      variable_configs: [],
    }),
    'variation.state': () => ({
      generation: generation++,
      axes: [],
      masters: [],
      instances: [instance('Regular')],
      first_glyph: null,
    }),
    'export.resolve_profile': (params) => resolved(params),
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
  await user.click(
    screen.getByRole('button', { name: 'Load instances and profiles' }),
  )
  await user.click(await screen.findByRole('checkbox', { name: 'Regular' }))
  await user.click(screen.getByRole('checkbox', { name: 'Web' }))
  await user.click(screen.getByRole('button', { name: 'Plan destinations' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Export batch' })).toBeEnabled(),
  )
  await user.click(screen.getByRole('button', { name: 'Export batch' }))
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The design space changed',
  )
  expect(transport.calls.some((call) => call.method === 'session.export')).toBe(
    false,
  )
})
