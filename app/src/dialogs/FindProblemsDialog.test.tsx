import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import {
  setTransport,
  type FindProblemsResult,
  type ProblemIssue,
} from '../api'
import { registerDocumentEditParticipant } from '../editor/documentEditParticipants'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { FindProblemsDialog } from './FindProblemsDialog'

const issue: ProblemIssue = {
  code: 'open-contour',
  severity: 'warning',
  message: 'The contour is open.',
  glyph: 7,
  glyph_name: 'A',
  master: 'm2',
  layer: 'layer-A',
  contour: 0,
  point: null,
  color_layer: null,
}

function report(
  overrides: Partial<FindProblemsResult> = {},
): FindProblemsResult {
  return {
    doc: 1,
    generation: 0,
    issues: [issue],
    total: 1,
    truncated: false,
    has_errors: false,
    ...overrides,
  }
}

function setup(
  result: () => FindProblemsResult | Promise<FindProblemsResult> = () =>
    report(),
) {
  const summary = makeSummary({
    masters: [
      { id: 'm1', name: 'Regular' },
      { id: 'm2', name: 'Bold' },
    ],
  })
  useDocumentStore.getState().ensure(summary.doc)
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
  useUiStore.getState().setDialog({ kind: 'findProblems', doc: summary.doc })
  const transport = createMockTransport({ 'doc.find_problems': result })
  setTransport(transport)
  const view = render(<FindProblemsDialog summary={summary} />)
  return { transport, ...view }
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
})

afterEach(() => setTransport(undefined))

describe('FindProblemsDialog', () => {
  it('shows severity/location and opens a glyph from the keyboard', async () => {
    const user = userEvent.setup()
    const { transport } = setup()
    const glyph = await screen.findByRole('button', { name: 'Open glyph A' })
    expect(screen.getByText('Warning')).toBeVisible()
    expect(screen.getByText(/Bold.*layer-A.*Contour 1/)).toBeVisible()
    expect(transport.calls).toEqual([
      { method: 'doc.find_problems', params: { doc: 1, limit: 200 } },
    ])
    glyph.focus()
    await user.keyboard('{Enter}')
    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    expect(
      useUiStore
        .getState()
        .tabs.find((tab) => tab.id === useUiStore.getState().activeTab),
    ).toMatchObject({ kind: 'glyph', doc: 1, gid: 7, label: 'A' })
    expect(useDocumentStore.getState().byDoc[1]?.master).toBe('m2')
  })

  it('settles pending drafts before scanning the document', async () => {
    let release!: () => void
    const gate = new Promise<void>((resolve) => {
      release = resolve
    })
    const unregister = registerDocumentEditParticipant(1, { flush: () => gate })
    try {
      const { transport } = setup()
      expect(transport.calls).toHaveLength(0)
      await act(async () => release())
      await screen.findByRole('button', { name: 'Open glyph A' })
      expect(transport.calls).toHaveLength(1)
    } finally {
      unregister()
    }
  })

  it('marks edited results stale and refreshes the current generation', async () => {
    const user = userEvent.setup()
    const { transport } = setup(() =>
      report({
        generation: useDocumentStore.getState().byDoc[1]?.generation ?? 0,
      }),
    )
    await screen.findByRole('button', { name: 'Open glyph A' })
    act(() => useDocumentStore.getState().noteGeneration(1, 2))
    expect(
      screen.getByText(
        'The document changed. Refresh to check the current version.',
      ),
    ).toBeVisible()
    expect(screen.getByRole('button', { name: 'Open glyph A' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Refresh' }))
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Open glyph A' }),
      ).toBeEnabled(),
    )
    expect(transport.calls).toHaveLength(2)
  })

  it('ignores a response when another document became active', async () => {
    let resolve!: (result: FindProblemsResult) => void
    const pending = new Promise<FindProblemsResult>((yes) => {
      resolve = yes
    })
    const { transport } = setup(() => pending)
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    await act(async () => {
      useUiStore.getState().openTab({ kind: 'font', doc: 2 })
      resolve(report())
      await pending
    })
    expect(
      screen.queryByRole('button', { name: 'Open glyph A' }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByText(
        'This report belongs to a different document. Reopen Find problems for the active font.',
      ),
    ).toBeVisible()
  })

  it('discards an older request when the dialog is retargeted', async () => {
    let resolve!: (result: FindProblemsResult) => void
    const pending = new Promise<FindProblemsResult>((yes) => {
      resolve = yes
    })
    const { rerender, transport } = setup(() => pending)
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    const next = makeSummary({ doc: 2 })
    setTransport(
      createMockTransport({
        'doc.find_problems': () => report({ doc: 2, issues: [], total: 0 }),
      }),
    )
    act(() => {
      useSessionStore.setState({ docs: [makeSummary(), next] })
      useUiStore.getState().openTab({ kind: 'font', doc: 2 })
    })
    rerender(<FindProblemsDialog summary={next} />)
    expect(await screen.findByText('No problems found.')).toBeVisible()
    await act(async () => {
      resolve(report())
      await pending
    })
    expect(
      screen.queryByRole('button', { name: 'Open glyph A' }),
    ).not.toBeInTheDocument()
    expect(screen.getByText('No problems found.')).toBeVisible()
  })

  it('resumes a discarded scan when the original document becomes active again', async () => {
    let resolve!: (result: FindProblemsResult) => void
    const pending = new Promise<FindProblemsResult>((yes) => {
      resolve = yes
    })
    let scans = 0
    const { transport } = setup(() =>
      ++scans === 1 ? pending : report({ issues: [], total: 0 }),
    )
    const originalTab = useUiStore.getState().activeTab
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    await act(async () => {
      useUiStore.getState().openTab({ kind: 'font', doc: 2 })
      resolve(report())
      await pending
    })
    act(() => {
      if (originalTab) useUiStore.getState().setActiveTab(originalTab)
    })
    expect(await screen.findByText('No problems found.')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Refresh' })).toBeEnabled()
    expect(transport.calls).toHaveLength(2)
  })

  it('reports errors and lets the user retry', async () => {
    const user = userEvent.setup()
    let failed = true
    setup(() => {
      if (failed) throw new Error('scanner disconnected')
      return report({ issues: [], total: 0 })
    })
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'scanner disconnected',
    )
    failed = false
    await user.click(screen.getByRole('button', { name: 'Refresh' }))
    expect(await screen.findByText('No problems found.')).toBeVisible()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('discloses truncation and offers a bounded result limit', async () => {
    const user = userEvent.setup()
    const { transport } = setup(() => report({ total: 300, truncated: true }))
    expect(await screen.findByText(/More problems were found/)).toBeVisible()
    await user.selectOptions(screen.getByLabelText('Result limit'), '2000')
    await waitFor(() => expect(transport.calls).toHaveLength(2))
    expect(transport.calls[1]?.params).toEqual({ doc: 1, limit: 2000 })
  })
})
