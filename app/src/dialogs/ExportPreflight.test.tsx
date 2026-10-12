import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

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
import { ExportPreflight } from './ExportPreflight'

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
  useDocumentStore.getState().ensure(1)
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: 1 })
  useUiStore.getState().setDialog({ kind: 'export', doc: 1 })
  const transport = createMockTransport({ 'doc.find_problems': result })
  setTransport(transport)
  const onNavigate = vi.fn()
  const view = render(
    <ExportPreflight doc={1} busy={false} onNavigate={onNavigate} />,
  )
  return { transport, onNavigate, ...view }
}

async function check() {
  await userEvent.click(
    screen.getByRole('button', { name: 'Check before export' }),
  )
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
})

afterEach(() => setTransport(undefined))

describe('ExportPreflight', () => {
  it('only scans explicitly and reports shown counts plus omitted errors', async () => {
    const { transport } = setup(() =>
      report({ total: 301, truncated: true, has_errors: true }),
    )
    expect(transport.calls).toHaveLength(0)
    await check()
    expect(
      await screen.findByText('0 errors · 1 warning · 0 notes shown'),
    ).toBeVisible()
    expect(
      screen.getByText('Showing 1 of at least 301 findings.'),
    ).toBeVisible()
    expect(
      screen.getByText('Errors were found. Review them before exporting.'),
    ).toBeVisible()
    expect(transport.calls).toEqual([
      { method: 'doc.find_problems', params: { doc: 1, limit: 200 } },
    ])
  })

  it('settles drafts and offers keyboard glyph navigation without document mutations', async () => {
    let release!: () => void
    const pending = new Promise<void>((resolve) => {
      release = resolve
    })
    const unregister = registerDocumentEditParticipant(1, {
      flush: () => pending,
    })
    try {
      const { transport, onNavigate } = setup()
      await check()
      expect(transport.calls).toHaveLength(0)
      await act(async () => release())
      const glyph = await screen.findByRole('button', { name: 'Open glyph A' })
      glyph.focus()
      await userEvent.keyboard('{Enter}')
      await waitFor(() => expect(onNavigate).toHaveBeenCalledOnce())
      expect(
        useUiStore
          .getState()
          .tabs.find((tab) => tab.id === useUiStore.getState().activeTab),
      ).toMatchObject({ kind: 'glyph', doc: 1, gid: 7, label: 'A' })
      expect(useDocumentStore.getState().byDoc[1]?.master).toBe('m2')
      expect(transport.calls).toHaveLength(1)
    } finally {
      unregister()
    }
  })

  it('displays actual feature diagnostics without creating a glyph shortcut', async () => {
    setup(() =>
      report({
        issues: [
          {
            ...issue,
            code: 'feature-error',
            severity: 'error',
            glyph: null,
            glyph_name: null,
            message: 'Line 3: unknown glyph Missing',
          },
        ],
        has_errors: true,
      }),
    )
    await check()
    expect(
      await screen.findByText('Line 3: unknown glyph Missing'),
    ).toBeVisible()
    expect(
      screen.getByText('1 error · 0 warnings · 0 notes shown'),
    ).toBeVisible()
    expect(
      screen.queryByRole('button', { name: 'Open glyph A' }),
    ).not.toBeInTheDocument()
  })

  it('disables stale glyph links and rechecks the current generation', async () => {
    const { transport } = setup(() =>
      report({
        generation: useDocumentStore.getState().byDoc[1]?.generation ?? 0,
      }),
    )
    await check()
    await screen.findByRole('button', { name: 'Open glyph A' })
    act(() => useDocumentStore.getState().noteGeneration(1, 2))
    expect(
      screen.getByText(
        'The document changed. Check again before using these findings.',
      ),
    ).toBeVisible()
    expect(screen.getByRole('button', { name: 'Open glyph A' })).toBeDisabled()
    await check()
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Open glyph A' }),
      ).toBeEnabled(),
    )
    expect(transport.calls).toHaveLength(2)
  })

  it('drops a delayed result when retargeted and requires another explicit check', async () => {
    let resolve!: (result: FindProblemsResult) => void
    const pending = new Promise<FindProblemsResult>((yes) => {
      resolve = yes
    })
    const { rerender, transport, onNavigate } = setup(() => pending)
    await check()
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    act(() => {
      useSessionStore.setState({
        docs: [makeSummary(), makeSummary({ doc: 2 })],
      })
      useUiStore.getState().openTab({ kind: 'font', doc: 2 })
    })
    rerender(<ExportPreflight doc={2} busy={false} onNavigate={onNavigate} />)
    await act(async () => {
      resolve(report())
      await pending
    })
    expect(
      screen.queryByRole('button', { name: 'Open glyph A' }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Check before export' }),
    ).toBeEnabled()
    expect(transport.calls).toHaveLength(1)
  })

  it('cancels a pending scan before transport when unmounted', async () => {
    let release!: () => void
    const pending = new Promise<void>((resolve) => {
      release = resolve
    })
    const unregister = registerDocumentEditParticipant(1, {
      flush: () => pending,
    })
    try {
      const { unmount, transport } = setup()
      await check()
      unmount()
      await act(async () => release())
      expect(transport.calls).toHaveLength(0)
    } finally {
      unregister()
    }
  })

  it('cancels pending navigation if the glyph report becomes stale while edits settle', async () => {
    const { onNavigate } = setup()
    await check()
    const glyph = await screen.findByRole('button', { name: 'Open glyph A' })
    let release!: () => void
    const pending = new Promise<void>((resolve) => {
      release = resolve
    })
    const unregister = registerDocumentEditParticipant(1, {
      flush: () => pending,
    })
    try {
      await userEvent.click(glyph)
      act(() => useDocumentStore.getState().noteGeneration(1, 1))
      await act(async () => release())
      expect(onNavigate).not.toHaveBeenCalled()
      expect(useUiStore.getState().tabs).toHaveLength(1)
    } finally {
      unregister()
    }
  })

  it('cancels pending navigation when the panel unmounts', async () => {
    const { onNavigate, unmount } = setup()
    await check()
    const glyph = await screen.findByRole('button', { name: 'Open glyph A' })
    let release!: () => void
    const pending = new Promise<void>((resolve) => {
      release = resolve
    })
    const unregister = registerDocumentEditParticipant(1, {
      flush: () => pending,
    })
    try {
      await userEvent.click(glyph)
      unmount()
      await act(async () => release())
      expect(onNavigate).not.toHaveBeenCalled()
      expect(useUiStore.getState().tabs).toHaveLength(1)
    } finally {
      unregister()
    }
  })

  it('reports scan errors and allows a successful retry', async () => {
    let failed = true
    setup(() => {
      if (failed) throw new Error('scanner disconnected')
      return report({ issues: [], total: 0 })
    })
    await check()
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'scanner disconnected',
    )
    failed = false
    await check()
    expect(await screen.findByText('No problems found.')).toBeVisible()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('prevents checks and navigation while exporting or after the document closes', async () => {
    const { rerender, onNavigate } = setup()
    await check()
    await screen.findByRole('button', { name: 'Open glyph A' })
    rerender(<ExportPreflight doc={1} busy onNavigate={onNavigate} />)
    expect(
      screen.getByRole('button', { name: 'Check before export' }),
    ).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Open glyph A' })).toBeDisabled()
    act(() => useSessionStore.setState({ docs: [] }))
    rerender(<ExportPreflight doc={1} busy={false} onNavigate={onNavigate} />)
    expect(
      screen.getByRole('button', { name: 'Check before export' }),
    ).toBeDisabled()
    expect(
      screen.getByText('Reopen Export for an active document to run checks.'),
    ).toBeVisible()
  })
})
