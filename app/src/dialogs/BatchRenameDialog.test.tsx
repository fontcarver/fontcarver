import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type GlyphCell } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { cellFor } from '../test/glyphFixtures'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../test/mockTransport'
import { BatchRenameDialog } from './BatchRenameDialog'

function setup(options: { fail?: boolean; names?: string[] } = {}) {
  let cells: GlyphCell[] = (options.names ?? ['A.alt', 'B.alt', 'C']).map(
    (name, gid) => cellFor(gid, { name }),
  )
  const transport = createMockTransport({
    'glyphs.list': () => ({ items: cells, total: cells.length }),
    'doc.apply': () => {
      if (options.fail) throw new Error('rename failed')
      return noChange([0, 1])
    },
    'doc.summary': () => makeSummary({ doc: 1 }),
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [makeSummary({ doc: 1 })] })
  useUiStore
    .getState()
    .setDialog({ kind: 'batchRename', doc: 1, glyphs: [0, 1] })
  render(<BatchRenameDialog doc={1} glyphs={[0, 1]} />)
  return {
    transport,
    replaceInventory: (next: GlyphCell[]) => {
      cells = next
    },
  }
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
})
afterEach(() => setTransport(undefined))

describe('BatchRenameDialog', () => {
  it('renames with a regular expression and its capture groups', async () => {
    const user = userEvent.setup()
    const { transport } = setup({ names: ['A.sc', 'B.sc'] })
    await user.click(screen.getByLabelText('Regular expression'))
    await user.type(screen.getByLabelText('Find'), '^(.)\\.sc$')
    await user.type(screen.getByLabelText('Replace with'), '$1.small')
    const preview = screen.getByRole('table', { name: 'Glyph rename preview' })
    expect(within(preview).getByText('A.small')).toBeInTheDocument()
    expect(within(preview).getByText('B.small')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Rename 2 glyphs' }))
    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    const applied = transport.calls.find(({ method }) => method === 'doc.apply')
      ?.params as { command: { Batch: unknown[] } }
    expect(applied.command.Batch).toHaveLength(4)
  })

  it('reports an unfinished pattern without staging anything', async () => {
    const user = userEvent.setup()
    setup({ names: ['A.sc', 'B.sc'] })
    await user.click(screen.getByLabelText('Regular expression'))
    await user.type(screen.getByLabelText('Find'), '(')
    expect(
      screen.getByText(/not a valid regular expression/),
    ).toBeInTheDocument()
    expect(screen.getByLabelText('Find')).toHaveAttribute(
      'aria-invalid',
      'true',
    )
    expect(screen.getByText('No glyph names would change.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^Rename/ })).toBeDisabled()
    // Completing the pattern clears the report and stages the rename again.
    await user.type(screen.getByLabelText('Find'), '.)\\.sc')
    await user.type(screen.getByLabelText('Replace with'), '$1')
    expect(
      screen.queryByText(/not a valid regular expression/),
    ).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Rename 2 glyphs' }),
    ).toBeEnabled()
  })

  it('matches loosely only while Ignore case is on', async () => {
    const user = userEvent.setup()
    setup({ names: ['Acute', 'Grave'] })
    await user.type(screen.getByLabelText('Find'), 'acute')
    expect(screen.getByText('No glyph names would change.')).toBeInTheDocument()
    await user.click(screen.getByLabelText('Ignore case'))
    await user.type(screen.getByLabelText('Replace with'), 'grave')
    const preview = screen.getByRole('table', { name: 'Glyph rename preview' })
    expect(within(preview).getByText('grave')).toBeInTheDocument()
  })

  it('previews selected names and applies one atomic batch', async () => {
    const user = userEvent.setup()
    const { transport } = setup()
    await user.type(screen.getByLabelText('Find'), '.alt')
    await user.type(screen.getByLabelText('Replace with'), '.ss01')
    const preview = screen.getByRole('table', { name: 'Glyph rename preview' })
    expect(within(preview).getByText('A.ss01')).toBeInTheDocument()
    expect(within(preview).queryByText('C')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Rename 2 glyphs' }))
    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    const commands = transport.calls.filter(
      ({ method }) => method === 'doc.apply',
    )
    expect(commands).toHaveLength(1)
    expect(commands[0]?.params).toEqual({
      doc: 1,
      label: 'Batch rename glyphs',
      command: {
        Batch: [
          {
            RenameGlyph: {
              from: 'A.alt',
              to: expect.stringMatching(/^_fcRename[0-9a-f]{32}_0$/),
            },
          },
          {
            RenameGlyph: {
              from: 'B.alt',
              to: expect.stringMatching(/^_fcRename[0-9a-f]{32}_1$/),
            },
          },
          {
            RenameGlyph: {
              from: expect.stringMatching(/^_fcRename[0-9a-f]{32}_0$/),
              to: 'A.ss01',
            },
          },
          {
            RenameGlyph: {
              from: expect.stringMatching(/^_fcRename[0-9a-f]{32}_1$/),
              to: 'B.ss01',
            },
          },
        ],
      },
    })
  })

  it('disables apply for collisions with an unselected glyph', async () => {
    const user = userEvent.setup()
    const { transport } = setup({ names: ['A.alt', 'B.alt', 'A'] })
    await user.type(screen.getByLabelText('Find'), '.alt')
    expect(screen.getByRole('alert')).toHaveTextContent('already used')
    expect(
      screen.getByRole('button', { name: 'Rename 2 glyphs' }),
    ).toBeDisabled()
    expect(transport.calls.some(({ method }) => method === 'doc.apply')).toBe(
      false,
    )
  })

  it('keeps the dialog and preview when apply fails', async () => {
    const user = userEvent.setup()
    setup({ fail: true })
    await user.type(screen.getByLabelText('Find'), '.alt')
    await user.click(screen.getByRole('button', { name: 'Rename 2 glyphs' }))
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Rename 2 glyphs' }),
      ).toBeEnabled(),
    )
    expect(useUiStore.getState().dialog?.kind).toBe('batchRename')
    expect(useUiStore.getState().toasts.at(-1)?.kind).toBe('error')
  })

  it('refreshes a stale preview without applying to changed names', async () => {
    const user = userEvent.setup()
    const { transport, replaceInventory } = setup()
    await user.type(screen.getByLabelText('Find'), '.alt')
    replaceInventory([
      cellFor(0, { name: 'renamed' }),
      cellFor(1, { name: 'B.alt' }),
    ])
    await user.click(screen.getByRole('button', { name: 'Rename 2 glyphs' }))
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent('inventory changed'),
    )
    expect(transport.calls.some(({ method }) => method === 'doc.apply')).toBe(
      false,
    )
    expect(
      screen.getByRole('button', { name: 'Rename 1 glyphs' }),
    ).toBeEnabled()
  })
})
