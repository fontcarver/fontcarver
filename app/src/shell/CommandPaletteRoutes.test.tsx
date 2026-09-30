import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it } from 'vitest'

import { setTransport, type GlyphCell } from '../api'
import {
  initialAutomationState,
  useAutomationStore,
} from '../automation/automationStore'
import {
  emptyFilter,
  initialDocumentState,
  useDocumentStore,
} from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { CommandPalette } from './CommandPalette'
import { resetCommands } from './commands'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'

beforeEach(() => {
  localStorage.clear()
  resetCommands()
  useUiStore.setState({ ...initialUiState(), paletteOpen: true })
  useDocumentStore.setState(initialDocumentState())
  useSessionStore.setState({ ...initialSessionState(), docs: [makeSummary()] })
  useAutomationStore.setState({ ...initialAutomationState(), loaded: true })
  useDocumentStore.getState().ensure(1)
  useUiStore.getState().openTab({ kind: 'font', doc: 1 })
})

afterEach(() => {
  setTransport(undefined)
  resetCommands()
})

const glyph: GlyphCell = {
  gid: 42,
  name: 'A.alt',
  unicodes: [65],
  category: 'Letter',
  sub_category: 'Uppercase',
  script: 'Latn',
  width: 600,
  bbox: null,
  path: null,
  color_label: null,
  export: true,
  has_components: false,
  is_empty: true,
  generation: 0,
}
const sidebar = {
  total: 1,
  unencoded: 0,
  scripts: [],
  categories: [],
  blocks: [],
  color_labels: [],
}

it('finds glyph names outside the active overview filter and opens the glyph tab', async () => {
  const user = userEvent.setup()
  const transport = createMockTransport({
    'glyphs.list': () => ({ total: 1, items: [glyph] }),
    'doc.sidebar': () => sidebar,
    'glyph.smart_filters.get': () => ({ filters: [] }),
  })
  setTransport(transport)
  useDocumentStore.getState().setFilter(1, { unencoded_only: true })
  render(<CommandPalette />)
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Search in' }),
    'glyphs',
  )
  await user.type(screen.getByRole('combobox', { name: 'Commands' }), 'A.alt')
  expect(
    await screen.findByRole('option', { name: 'A.alt U+0041' }),
  ).toBeInTheDocument()
  await user.keyboard('{Enter}')
  await waitFor(() =>
    expect(useUiStore.getState().activeTab).toBe(tabId('glyph', 1, 42)),
  )
  expect(useUiStore.getState().paletteOpen).toBe(false)
  expect(
    transport.calls.find((call) => call.method === 'glyphs.list')?.params,
  ).toMatchObject({ filter: { unencoded_only: false } })
})

it('finds a saved smart filter and replaces the overview filter after selection', async () => {
  const user = userEvent.setup()
  setTransport(
    createMockTransport({
      'doc.sidebar': () => sidebar,
      'glyph.smart_filters.get': () => ({
        filters: [{ name: 'Review Latin', expression: 'script == "Latn"' }],
      }),
    }),
  )
  render(<CommandPalette />)
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Search in' }),
    'filters',
  )
  await user.type(screen.getByRole('combobox', { name: 'Commands' }), 'Review')
  await user.click(
    await screen.findByRole('option', {
      name: 'Review Latin Saved filter',
    }),
  )
  await waitFor(() =>
    expect(useDocumentStore.getState().byDoc[1]?.filter).toEqual({
      ...emptyFilter(),
      smart_expression: 'script == "Latn"',
    }),
  )
  expect(useUiStore.getState().activeTab).toBe(tabId('font', 1))
})

it('opens a discovered script in the console without executing it', async () => {
  const user = userEvent.setup()
  useAutomationStore.setState({
    scripts: [
      {
        name: 'inspect-widths.js',
        folder: 'configured',
        language: 'javascript',
        bytes: 12,
      },
    ],
  })
  const transport = createMockTransport({
    'doc.sidebar': () => sidebar,
    'glyph.smart_filters.get': () => ({ filters: [] }),
    'automation.script_read': () => ({
      name: 'inspect-widths.js',
      folder: 'configured' as const,
      language: 'javascript' as const,
      source: 'console.log(1)',
    }),
  })
  setTransport(transport)
  render(<CommandPalette />)
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Search in' }),
    'scripts',
  )
  await user.type(screen.getByRole('combobox', { name: 'Commands' }), 'widths')
  const options = within(screen.getByRole('listbox')).getAllByRole('option')
  expect(options).toHaveLength(1)
  await user.keyboard('{Enter}')
  await waitFor(() =>
    expect(useAutomationStore.getState().selectedScript).toBe(
      'inspect-widths.js',
    ),
  )
  expect(useAutomationStore.getState()).toMatchObject({
    open: true,
    running: false,
    source: 'console.log(1)',
  })
  expect(transport.calls.map((call) => call.method)).not.toContain(
    'automation.python_run',
  )
})

it('groups results of every category into labelled sections', async () => {
  const user = userEvent.setup()
  useAutomationStore.setState({
    scripts: [
      {
        name: 'review.js',
        folder: 'configured',
        language: 'javascript',
        bytes: 12,
      },
    ],
  })
  setTransport(
    createMockTransport({
      'glyphs.list': () => ({
        total: 1,
        items: [{ ...glyph, name: 'review' }],
      }),
      'doc.sidebar': () => sidebar,
      'glyph.smart_filters.get': () => ({
        filters: [{ name: 'review latin', expression: 'script == "Latn"' }],
      }),
    }),
  )
  render(<CommandPalette />)
  await user.type(screen.getByRole('combobox', { name: 'Commands' }), 'review')

  const listbox = screen.getByRole('listbox')
  // The glyph section arrives after its debounced metadata query.
  await within(listbox).findByRole('group', { name: 'Glyphs' })
  const groups = within(listbox).getAllByRole('group')
  // Sections appear in the documented order and each is named for its category.
  const names = groups.map((group) => group.getAttribute('aria-label'))
  expect(names).toEqual(
    ['Filters', 'Scripts', 'Glyphs'].filter((name) => names.includes(name)),
  )
  expect(names).toContain('Glyphs')
  expect(
    within(groups[names.indexOf('Glyphs')]).getAllByRole('option').length,
  ).toBe(1)
})

it('walks the visible rows, not the ranked order behind the sections', async () => {
  const user = userEvent.setup()
  // Commands, filters and glyphs all match, so the sections reorder the ranked
  // list — which is exactly when the highlight used to disagree with the rows.
  registerWorkspaceTabCommands()
  setTransport(
    createMockTransport({
      'glyphs.list': () => ({
        total: 2,
        items: [
          { ...glyph, gid: 1, name: 'arab' },
          { ...glyph, gid: 2, name: 'armn' },
        ],
      }),
      'doc.sidebar': () => sidebar,
      'glyph.smart_filters.get': () => ({ filters: [] }),
    }),
  )
  render(<CommandPalette />)
  await user.type(screen.getByRole('combobox', { name: 'Commands' }), 'a')

  const listbox = await screen.findByRole('listbox')
  await within(listbox).findByRole('group', { name: 'Glyphs' })
  expect(within(listbox).getAllByRole('group').length).toBeGreaterThan(1)
  const rows = within(listbox).getAllByRole('option')
  expect(rows.length).toBeGreaterThan(4)

  // The first row starts highlighted, and each ArrowDown moves to the next row
  // the user can actually see.
  expect(rows[0]).toHaveAttribute('aria-selected', 'true')
  for (let index = 1; index < Math.min(rows.length, 5); index += 1) {
    await user.keyboard('{ArrowDown}')
    const current = within(listbox).getAllByRole('option')
    expect(current[index]).toHaveAttribute('aria-selected', 'true')
    expect(current[index - 1]).toHaveAttribute('aria-selected', 'false')
  }

  // Exactly one visible row is highlighted, and it is the fifth one.
  const walked = within(listbox).getAllByRole('option')
  const highlighted = walked.filter(
    (row) => row.getAttribute('aria-selected') === 'true',
  )
  expect(highlighted).toHaveLength(1)
  expect(walked.indexOf(highlighted[0])).toBe(4)
})

it('runs the row the keyboard highlighted and nothing else', async () => {
  const user = userEvent.setup()
  setTransport(
    createMockTransport({
      'glyphs.list': () => ({
        total: 1,
        items: [{ ...glyph, gid: 42, name: 'arab' }],
      }),
      'doc.sidebar': () => sidebar,
      'glyph.smart_filters.get': () => ({ filters: [] }),
    }),
  )
  render(<CommandPalette />)
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Search in' }),
    'glyphs',
  )
  await user.type(screen.getByRole('combobox', { name: 'Commands' }), 'arab')
  const listbox = await screen.findByRole('listbox')
  await within(listbox).findByRole('option', { name: /^arab/ })

  await user.keyboard('{Enter}')
  await waitFor(() =>
    expect(useUiStore.getState().activeTab).toBe(tabId('glyph', 1, 42)),
  )
})
