import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport, type GlyphListResult } from '../api'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import {
  captureWorkspace,
  useWorkspaceStore,
  workspaceDocumentKey,
  type WorkspaceSnapshot,
} from '../stores/workspaceStore'
import { makeCell } from '../test/fontFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { prepareTabTransition } from './tabActions'
import { WorkspaceDialogHost } from './WorkspaceDialogHost'

vi.mock('./tabActions', () => ({ prepareTabTransition: vi.fn() }))

const first = makeSummary({ path: '/fonts/first.fcf' })
const second = makeSummary({ doc: 2, path: '/fonts/second.fcf' })
const key = workspaceDocumentKey(first)
const glyphs: GlyphListResult = {
  total: 1,
  items: [makeCell({ gid: 91, name: 'A' })],
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((done, fail) => {
    resolve = done
    reject = fail
  })
  return { promise, resolve, reject }
}

function savedLayout(): WorkspaceSnapshot {
  return {
    ...captureWorkspace(first.doc, initialUiState()),
    tabs: [{ kind: 'font' }, { kind: 'glyph', glyphName: 'A' }],
    activeTab: 1,
    panels: { sidebar: false, inspector: true },
    sizes: { sidebar: 310, inspector: 390 },
    sections: { metrics: false },
    textPreviewVisible: false,
  }
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState({ ...initialSessionState(), docs: [first, second] })
  useDocumentStore.setState(initialDocumentState())
  useWorkspaceStore.setState({ documents: {}, dialogOpen: true })
  useUiStore.getState().openTab({ kind: 'font', doc: 1 })
  useUiStore.getState().openTab({ kind: 'font', doc: 2 }, false)
  useUiStore.getState().openTab({ kind: 'features', doc: 1 })
  useUiStore.getState().openTab({ kind: 'bitmap', doc: 2 }, false)
  useWorkspaceStore.getState().savePreset(key, 'Drawing', savedLayout())
  vi.mocked(prepareTabTransition).mockReturnValue({
    ready: Promise.resolve(true),
    settle: vi.fn().mockResolvedValue(true),
    isCurrent: () => true,
  })
})

afterEach(() => {
  setTransport(undefined)
  vi.resetAllMocks()
})

it('waits for both edit barriers and remaps glyph names while preserving other documents', async () => {
  const ready = deferred<boolean>()
  const settled = deferred<boolean>()
  const settle = vi.fn(() => settled.promise)
  vi.mocked(prepareTabTransition).mockReturnValue({
    ready: ready.promise,
    settle,
    isCurrent: () => true,
  })
  const transport = createMockTransport({ 'glyphs.list': () => glyphs })
  setTransport(transport)
  const original = useUiStore.getState().tabs
  const unrelated = original.filter((tab) => tab.doc === 2)
  render(<WorkspaceDialogHost />)

  await userEvent.click(screen.getByRole('button', { name: 'Apply Drawing' }))
  expect(useUiStore.getState().tabs).toEqual(original)
  expect(transport.calls).toEqual([])
  await act(async () => ready.resolve(true))
  await waitFor(() => expect(settle).toHaveBeenCalledOnce())
  expect(useUiStore.getState().tabs).toEqual(original)
  expect(useWorkspaceStore.getState().getCurrent(key)).toBeNull()

  await act(async () => settled.resolve(true))

  expect(useUiStore.getState()).toMatchObject({
    activeTab: tabId('glyph', 1, 91),
    panels: { sidebar: false, inspector: true },
    sizes: { sidebar: 310, inspector: 390 },
    sections: { metrics: false },
    textPreviewVisible: false,
  })
  expect(useUiStore.getState().tabs.filter((tab) => tab.doc === 1)).toEqual([
    { id: tabId('font', 1), kind: 'font', doc: 1 },
    { id: tabId('glyph', 1, 91), kind: 'glyph', doc: 1, gid: 91 },
  ])
  expect(useUiStore.getState().tabs.filter((tab) => tab.doc === 2)).toEqual(
    unrelated,
  )
  expect(useWorkspaceStore.getState().getCurrent(key)).toEqual(savedLayout())
})

it.each(['ready', 'settle'] as const)(
  'preserves the open tabs and saved current workspace when the %s barrier fails',
  async (barrier) => {
    vi.mocked(prepareTabTransition).mockReturnValue({
      ready: Promise.resolve(barrier !== 'ready'),
      settle: vi.fn().mockResolvedValue(barrier !== 'settle'),
      isCurrent: () => true,
    })
    setTransport(createMockTransport({ 'glyphs.list': () => glyphs }))
    const original = useUiStore.getState().tabs
    render(<WorkspaceDialogHost />)

    await userEvent.click(screen.getByRole('button', { name: 'Apply Drawing' }))

    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Apply Drawing' }),
      ).toBeEnabled(),
    )
    expect(useUiStore.getState().tabs).toEqual(original)
    expect(useUiStore.getState().activeTab).toBe(tabId('features', 1))
    expect(useWorkspaceStore.getState().getCurrent(key)).toBeNull()
  },
)

it('reports a delayed glyph lookup failure without replacing the open tabs', async () => {
  const lookup = deferred<GlyphListResult>()
  const transport = createMockTransport({ 'glyphs.list': () => lookup.promise })
  setTransport(transport)
  const original = useUiStore.getState().tabs
  render(<WorkspaceDialogHost />)
  await userEvent.click(screen.getByRole('button', { name: 'Apply Drawing' }))
  await waitFor(() => expect(transport.calls).toHaveLength(1))

  await act(async () => lookup.reject(new Error('glyph lookup unavailable')))

  expect(useUiStore.getState().tabs).toEqual(original)
  expect(useUiStore.getState().toasts).toEqual([
    expect.objectContaining({ kind: 'error' }),
  ])
  expect(useWorkspaceStore.getState().getCurrent(key)).toBeNull()
  expect(screen.getByRole('button', { name: 'Apply Drawing' })).toBeEnabled()
})

it.each(['navigate', 'same-document', 'close', 'dismiss'] as const)(
  'discards a delayed apply after the user chooses to %s the document',
  async (action) => {
    const lookup = deferred<GlyphListResult>()
    const transport = createMockTransport({
      'glyphs.list': () => lookup.promise,
    })
    setTransport(transport)
    const initialTab = tabId('features', 1)
    vi.mocked(prepareTabTransition).mockReturnValue({
      ready: Promise.resolve(true),
      settle: vi.fn().mockResolvedValue(true),
      isCurrent: () => useUiStore.getState().activeTab === initialTab,
    })
    render(<WorkspaceDialogHost />)
    await userEvent.click(screen.getByRole('button', { name: 'Apply Drawing' }))
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    const target =
      action === 'dismiss'
        ? initialTab
        : tabId('font', action === 'same-document' ? 1 : 2)
    act(() => {
      useUiStore.getState().setActiveTab(target)
      if (action === 'dismiss')
        useWorkspaceStore.getState().setDialogOpen(false)
      if (action === 'close') {
        useUiStore.getState().closeTabsForDoc(1)
        useSessionStore.setState({ docs: [second] })
      }
    })
    const navigated = useUiStore.getState().tabs

    await act(async () => lookup.resolve(glyphs))

    expect(useUiStore.getState().tabs).toEqual(navigated)
    expect(useUiStore.getState().activeTab).toBe(target)
    expect(useWorkspaceStore.getState().getCurrent(key)).toBeNull()
  },
)
