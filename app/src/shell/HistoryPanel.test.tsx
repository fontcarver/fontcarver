import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it } from 'vitest'

import { setTransport, type DocumentHistory } from '../api'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { HistoryPanel } from './HistoryPanel'

const entry = (
  index: number,
  label: string,
  extra: Partial<DocumentHistory['entries'][number]> = {},
): DocumentHistory['entries'][number] => ({
  index,
  label,
  glyphs: [],
  layers: [],
  strikes: [],
  font_level: false,
  undone: false,
  ...extra,
})

const history: DocumentHistory = {
  cursor: 2,
  len: 3,
  truncated: false,
  entries: [
    entry(0, 'Set advance', {
      glyphs: ['A'],
      layers: [{ name: 'Regular', role: null }],
    }),
    entry(1, 'Rename family', { font_level: true }),
    entry(2, 'Paint pixels', {
      glyphs: ['B'],
      strikes: ['16px/1bpp'],
      undone: true,
    }),
  ],
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
  useSessionStore.setState({ ...initialSessionState(), docs: [makeSummary()] })
  useDocumentStore.getState().ensure(1)
  useUiStore.getState().setSection('history', true)
})

afterEach(() => setTransport(undefined))

it('lists each gesture with its scope and marks the current position', async () => {
  setTransport(createMockTransport({ 'doc.history': () => history }))
  render(<HistoryPanel doc={1} />)

  const list = await screen.findByRole('list', { name: 'History entries' })
  const rows = list.querySelectorAll('button')
  // Newest first, with the "document as opened" row last.
  expect([...rows].map((row) => row.textContent)).toEqual([
    'Paint pixelsGlyphs: B · Strikes: 16px/1bppundone',
    'Rename familyFont-levelcurrent',
    'Set advanceGlyphs: A · Layers: Regular',
    'Document as opened',
  ])
  expect(rows[1]).toHaveAttribute('aria-current', 'true')
  expect(rows[0]).not.toHaveAttribute('aria-current')
  expect(rows[2]).toHaveAttribute('aria-label', 'Go to Set advance')
})

it('moves the cursor to the selected entry, stating the length it saw', async () => {
  const user = userEvent.setup()
  const transport = createMockTransport({
    'doc.history': () => history,
    'doc.history_go': () => ({ ...history, cursor: 1 }),
    'doc.summary': () => makeSummary(),
  })
  setTransport(transport)
  render(<HistoryPanel doc={1} />)

  await user.click(
    await screen.findByRole('button', { name: 'Go to Set advance' }),
  )
  await waitFor(() =>
    expect(
      transport.calls.find((call) => call.method === 'doc.history_go')?.params,
    ).toEqual({ doc: 1, cursor: 1, expected_len: 3 }),
  )
})

it('returns to the state before any edit', async () => {
  const user = userEvent.setup()
  const transport = createMockTransport({
    'doc.history': () => history,
    'doc.history_go': () => ({ ...history, cursor: 0 }),
    'doc.summary': () => makeSummary(),
  })
  setTransport(transport)
  render(<HistoryPanel doc={1} />)

  await user.click(
    await screen.findByRole('button', {
      name: 'Go back to the state before any edit',
    }),
  )
  await waitFor(() =>
    expect(
      transport.calls.find((call) => call.method === 'doc.history_go')?.params,
    ).toMatchObject({ cursor: 0 }),
  )
  await waitFor(() =>
    expect(
      screen.getByRole('button', {
        name: 'Go back to the state before any edit',
      }),
    ).toBeDisabled(),
  )
})

it('reloads and says so when the timeline moved under the list', async () => {
  const user = userEvent.setup()
  let reloads = 0
  setTransport(
    createMockTransport({
      'doc.history': () => {
        reloads += 1
        return history
      },
      'doc.history_go': () => {
        throw new Error('the history has 4 entries, not the 3 …')
      },
    }),
  )
  render(<HistoryPanel doc={1} />)

  await user.click(
    await screen.findByRole('button', { name: 'Go to Set advance' }),
  )
  expect(
    await screen.findByText(
      'The history changed while this list was open. It has been reloaded.',
    ),
  ).toBeInTheDocument()
  // The document is left exactly as it was and the list is refreshed.
  await waitFor(() => expect(reloads).toBeGreaterThan(1))
  expect(useUiStore.getState().toasts).toHaveLength(1)
})

it('says nothing has been edited when the timeline is empty', async () => {
  setTransport(
    createMockTransport({
      'doc.history': () => ({
        entries: [],
        cursor: 0,
        len: 0,
        truncated: false,
      }),
    }),
  )
  render(<HistoryPanel doc={1} />)
  expect(
    await screen.findByText('Nothing has been edited yet.'),
  ).toBeInTheDocument()
  expect(screen.queryByRole('list')).not.toBeInTheDocument()
})

it('reads the timeline only while its section is open', async () => {
  const transport = createMockTransport({ 'doc.history': () => history })
  setTransport(transport)
  useUiStore.getState().setSection('history', false)
  render(<HistoryPanel doc={1} />)

  expect(transport.calls).toHaveLength(0)
  useUiStore.getState().setSection('history', true)
  await screen.findByRole('list', { name: 'History entries' })
  expect(
    transport.calls.filter((call) => call.method === 'doc.history'),
  ).toHaveLength(1)
})

it('names an unnamed layer by its role, in the reader language', async () => {
  setTransport(
    createMockTransport({
      'doc.history': () => ({
        cursor: 1,
        len: 1,
        truncated: false,
        entries: [
          entry(0, 'Set advance', {
            glyphs: ['A'],
            // The kernel sends a role identifier, never English prose.
            layers: [{ name: null, role: 'background' }],
          }),
        ],
      }),
    }),
  )
  render(<HistoryPanel doc={1} />)

  const list = await screen.findByRole('list', { name: 'History entries' })
  expect(list.querySelectorAll('button')[0]?.textContent).toContain(
    'Layers: Background layer',
  )
})

it('falls back to a generic layer label for a role it does not know', async () => {
  setTransport(
    createMockTransport({
      'doc.history': () => ({
        cursor: 1,
        len: 1,
        truncated: false,
        entries: [
          entry(0, 'Set advance', {
            glyphs: ['A'],
            layers: [{ name: null, role: 'from-a-later-build' }],
          }),
        ],
      }),
    }),
  )
  render(<HistoryPanel doc={1} />)

  const list = await screen.findByRole('list', { name: 'History entries' })
  expect(list.querySelectorAll('button')[0]?.textContent).toContain(
    'Layers: Layer',
  )
})
