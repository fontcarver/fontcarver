import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport } from '../../api'
import { afterDocumentEdits, holdDocumentEdits } from '../../editor/editSession'
import { i18n } from '../../i18n'
import {
  createMockTransport,
  noChange,
  type MockHandlers,
} from '../../test/mockTransport'
import { SmartFiltersPanel } from './SmartFiltersPanel'

type Filter = { name: string; expression: string }
type Filters = { filters: Filter[] }

vi.mock('./SmartFilterEditor', () => ({
  SmartFilterEditor: ({
    initialFilter,
    onSave,
    onCancel,
  }: {
    initialFilter?: Filter
    onSave: (filter: Filter) => Promise<void>
    onCancel: () => void
  }) => (
    <form
      aria-label="Filter editor"
      onSubmit={(event) => {
        event.preventDefault()
        const data = new FormData(event.currentTarget)
        void onSave({
          name: String(data.get('name')),
          expression: String(data.get('expression')),
        }).catch(() => undefined)
      }}
    >
      <input
        aria-label="Filter name"
        name="name"
        defaultValue={initialFilter?.name ?? ''}
      />
      <input
        aria-label="Expression"
        name="expression"
        defaultValue={initialFilter?.expression ?? ''}
      />
      <button type="submit">Save</button>
      <button type="button" onClick={onCancel}>
        Cancel
      </button>
    </form>
  ),
}))

const initial = [{ name: 'Wide', expression: 'width >= 600' }]

function setup(extra: MockHandlers = {}) {
  let filters = initial
  const mock = createMockTransport({
    'glyph.smart_filters.get': () => ({ filters }),
    'glyph.smart_filters.set': (params) => {
      filters = params.filters
      return { filters }
    },
    ...extra,
  })
  setTransport(mock)
  return mock
}

beforeEach(() => {
  i18n.addResourceBundle(
    'en',
    'translation',
    {
      smartFilters: {
        title: 'Smart filters',
        new: 'New filter',
        edit: 'Edit filter',
        delete: 'Delete filter',
        clear: 'Clear filter',
        loading: 'Loading filters…',
        loadError: 'Could not load filters.',
        retry: 'Retry',
        saveError: 'Could not save filters.',
        noFilter: 'No smart filter',
        customFilter: 'Custom expression',
        savedFilters: 'Saved filters',
      },
    },
    true,
    true,
  )
})

afterEach(() => setTransport(undefined))

async function loaded() {
  await screen.findByRole('option', { name: 'Wide' })
  return screen.getByRole('combobox', { name: 'Saved filters' })
}

function edit(name: string, expression: string) {
  fireEvent.change(screen.getByRole('textbox', { name: 'Filter name' }), {
    target: { value: name },
  })
  fireEvent.change(screen.getByRole('textbox', { name: 'Expression' }), {
    target: { value: expression },
  })
  fireEvent.click(screen.getByRole('button', { name: 'Save' }))
}

it('loads named filters, applies the expression and clears it', async () => {
  setup()
  const change = vi.fn()
  const view = render(
    <SmartFiltersPanel doc={1} expression={null} onExpressionChange={change} />,
  )
  const select = await loaded()
  const option = screen.getByRole('option', {
    name: 'Wide',
  }) as HTMLOptionElement
  fireEvent.change(select, { target: { value: option.value } })
  expect(change).toHaveBeenLastCalledWith('width >= 600')
  view.rerender(
    <SmartFiltersPanel
      doc={1}
      expression="width >= 600"
      onExpressionChange={change}
    />,
  )
  fireEvent.click(screen.getByRole('button', { name: 'Clear filter' }))
  expect(change).toHaveBeenLastCalledWith(null)
})

it('persists new, edited and deleted filters and updates the applied expression', async () => {
  const mock = setup()
  const change = vi.fn()
  const view = render(
    <SmartFiltersPanel
      doc={1}
      expression="width >= 600"
      onExpressionChange={change}
    />,
  )
  await loaded()
  fireEvent.click(screen.getByRole('button', { name: 'Edit filter' }))
  edit('Very wide', 'width >= 900')
  await screen.findByRole('option', { name: 'Very wide' })
  expect(change).toHaveBeenLastCalledWith('width >= 900')
  expect(
    mock.calls.find((entry) => entry.method === 'glyph.smart_filters.set')
      ?.params,
  ).toEqual({
    doc: 1,
    filters: [{ name: 'Very wide', expression: 'width >= 900' }],
  })
  view.rerender(
    <SmartFiltersPanel
      doc={1}
      expression="width >= 900"
      onExpressionChange={change}
    />,
  )
  fireEvent.click(screen.getByRole('button', { name: 'New filter' }))
  edit('Red', 'color == 1')
  await screen.findByRole('option', { name: 'Red' })
  expect(change).toHaveBeenLastCalledWith('color == 1')
  view.rerender(
    <SmartFiltersPanel
      doc={1}
      expression="color == 1"
      onExpressionChange={change}
    />,
  )
  fireEvent.click(screen.getByRole('button', { name: 'Delete filter' }))
  await waitFor(() =>
    expect(
      screen.queryByRole('option', { name: 'Red' }),
    ).not.toBeInTheDocument(),
  )
  expect(change).toHaveBeenLastCalledWith(null)
  expect(
    mock.calls
      .filter((entry) => entry.method === 'glyph.smart_filters.set')
      .at(-1)?.params,
  ).toEqual({
    doc: 1,
    filters: [{ name: 'Very wide', expression: 'width >= 900' }],
  })
})

it('keeps saved state and editor available after a failed save', async () => {
  setup({
    'glyph.smart_filters.set': () => {
      throw new Error('invalid expression')
    },
  })
  const change = vi.fn()
  render(
    <SmartFiltersPanel
      doc={1}
      expression="width >= 600"
      onExpressionChange={change}
    />,
  )
  await loaded()
  fireEvent.click(screen.getByRole('button', { name: 'Edit filter' }))
  edit('Bad', 'width ???')
  expect(await screen.findByText('Could not save filters.')).toBeVisible()
  expect(screen.getByRole('form', { name: 'Filter editor' })).toBeVisible()
  expect(screen.getByRole('option', { name: 'Wide' })).toBeInTheDocument()
  expect(change).not.toHaveBeenCalled()
})

it('surfaces a load failure, preserves a custom expression and retries', async () => {
  let failed = true
  setup({
    'glyph.smart_filters.get': () => {
      if (failed) throw new Error('unavailable')
      return { filters: initial }
    },
  })
  const change = vi.fn()
  render(
    <SmartFiltersPanel
      doc={1}
      expression="has_note"
      onExpressionChange={change}
    />,
  )
  expect(await screen.findByText('Could not load filters.')).toBeVisible()
  expect(screen.getByRole('button', { name: 'New filter' })).toBeDisabled()
  fireEvent.click(screen.getByRole('button', { name: 'Clear filter' }))
  expect(change).toHaveBeenLastCalledWith(null)
  failed = false
  fireEvent.click(screen.getByRole('button', { name: 'Retry' }))
  await loaded()
  expect(screen.getByRole('button', { name: 'New filter' })).toBeEnabled()
  expect(screen.queryByText('Could not load filters.')).not.toBeInTheDocument()
})

it('reloads on this document’s font-info changes and ignores older replies', async () => {
  let resolveOld!: (value: Filters) => void
  let requests = 0
  const mock = setup({
    'glyph.smart_filters.get': () =>
      ++requests === 1
        ? new Promise((resolve) => {
            resolveOld = resolve
          })
        : { filters: [{ name: 'Restored', expression: 'color == 3' }] },
  })
  render(
    <SmartFiltersPanel
      doc={1}
      expression={null}
      onExpressionChange={vi.fn()}
    />,
  )
  const event = {
    doc: 1,
    generation: 2,
    change: { ...noChange(), font_info: true },
    undo_label: null,
    redo_label: 'Filter edit',
    dirty: false,
  }
  act(() => mock.emit({ ...event, doc: 2 }))
  act(() => mock.emit({ ...event, change: noChange() }))
  expect(requests).toBe(1)
  act(() => mock.emit(event))
  await screen.findByRole('option', { name: 'Restored' })
  await act(async () => resolveOld({ filters: initial }))
  expect(screen.queryByRole('option', { name: 'Wide' })).not.toBeInTheDocument()
})

it('ignores reads and closes an open editor when the document changes', async () => {
  let resolveOld!: (value: Filters) => void
  setup({
    'glyph.smart_filters.get': ({ doc }) =>
      doc === 1
        ? new Promise((resolve) => {
            resolveOld = resolve
          })
        : { filters: [{ name: 'Other font', expression: 'color == 2' }] },
  })
  const change = vi.fn()
  const view = render(
    <SmartFiltersPanel doc={1} expression={null} onExpressionChange={change} />,
  )
  view.rerender(
    <SmartFiltersPanel doc={2} expression={null} onExpressionChange={change} />,
  )
  await screen.findByRole('option', { name: 'Other font' })
  await act(async () => resolveOld({ filters: initial }))
  expect(screen.queryByRole('option', { name: 'Wide' })).not.toBeInTheDocument()
  fireEvent.click(screen.getByRole('button', { name: 'New filter' }))
  expect(screen.getByRole('form', { name: 'Filter editor' })).toBeVisible()
  view.rerender(
    <SmartFiltersPanel doc={3} expression={null} onExpressionChange={change} />,
  )
  expect(
    screen.queryByRole('form', { name: 'Filter editor' }),
  ).not.toBeInTheDocument()
})

it('does not apply a completed save to a different document', async () => {
  let resolveSave!: (value: Filters) => void
  setup({
    'glyph.smart_filters.set': () =>
      new Promise((resolve) => {
        resolveSave = resolve
      }),
  })
  const change = vi.fn()
  const view = render(
    <SmartFiltersPanel doc={1} expression={null} onExpressionChange={change} />,
  )
  await loaded()
  fireEvent.click(screen.getByRole('button', { name: 'New filter' }))
  edit('Saved elsewhere', 'has_note')
  await waitFor(() => expect(resolveSave).toBeDefined())
  view.rerender(
    <SmartFiltersPanel doc={2} expression={null} onExpressionChange={change} />,
  )
  await loaded()
  await act(async () =>
    resolveSave({
      filters: [{ name: 'Saved elsewhere', expression: 'has_note' }],
    }),
  )
  expect(change).not.toHaveBeenCalled()
  expect(
    screen.queryByRole('option', { name: 'Saved elsewhere' }),
  ).not.toBeInTheDocument()
})

it('orders filter persistence behind pending edits and blocks save until it completes', async () => {
  let releaseEarlier!: () => void
  let releaseFilter!: (value: Filters) => void
  const mock = setup({
    'glyph.smart_filters.set': () =>
      new Promise((resolve) => {
        releaseFilter = resolve
      }),
  })
  render(
    <SmartFiltersPanel
      doc={1}
      expression="width >= 600"
      onExpressionChange={vi.fn()}
    />,
  )
  await loaded()
  holdDocumentEdits(
    1,
    new Promise<void>((resolve) => {
      releaseEarlier = resolve
    }),
  )
  fireEvent.click(screen.getByRole('button', { name: 'Delete filter' }))
  const saved = vi.fn()
  const saving = afterDocumentEdits(1, async () => {
    saved()
  })
  expect(
    mock.calls.filter((entry) => entry.method === 'glyph.smart_filters.set'),
  ).toHaveLength(0)
  await act(async () => releaseEarlier())
  await waitFor(() => expect(releaseFilter).toBeDefined())
  expect(saved).not.toHaveBeenCalled()
  await act(async () => {
    releaseFilter({ filters: [] })
    await saving
  })
  expect(saved).toHaveBeenCalledOnce()
})
