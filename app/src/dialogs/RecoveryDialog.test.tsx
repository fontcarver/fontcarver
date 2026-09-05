import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type RecoveryEntry } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { checkForRecovery } from './recovery'
import { RecoveryDialog } from './RecoveryDialog'

const ENTRIES: RecoveryEntry[] = [
  {
    doc_path: '/fonts/Workshop.fwf',
    autosave_path: '/state/autosave/1.fwf',
    saved_at_ms: 1_700_000_000_000,
  },
  {
    doc_path: null,
    autosave_path: '/state/autosave/2.fwf',
    saved_at_ms: 1_700_000_060_000,
  },
]

function setup() {
  const transport = createMockTransport({
    'session.recover': () =>
      makeSummary({
        doc: 3,
        path: '/fonts/Workshop.fwf',
        dirty: true,
        recovered_from: '/fonts/Workshop.fwf',
      }),
    'session.discard_recovery': () => null,
  })
  setTransport(transport)
  useUiStore.getState().setDialog({ kind: 'recovery', entries: ENTRIES })
  render(<RecoveryDialog entries={ENTRIES} />)
  return transport
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('RecoveryDialog', () => {
  it('lists every snapshot with its original path', () => {
    setup()

    expect(screen.getByText('/fonts/Workshop.fwf')).toBeInTheDocument()
    expect(screen.getByText('Untitled document')).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: 'Recover' })).toHaveLength(2)
  })

  it('recovers a snapshot into a tab and marks the document recovered', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getAllByRole('button', { name: 'Recover' })[0]!)

    await waitFor(() => expect(useSessionStore.getState().docs).toHaveLength(1))
    expect(transport.calls[0]).toEqual({
      method: 'session.recover',
      params: { autosave_path: '/state/autosave/1.fwf' },
    })
    expect(useSessionStore.getState().docs[0]?.recovered_from).toBe(
      '/fonts/Workshop.fwf',
    )
    expect(useUiStore.getState().tabs.map((tab) => tab.id)).toEqual(['font:3'])
  })

  it('asks before discarding a snapshot', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getAllByRole('button', { name: 'Discard' })[0]!)
    expect(transport.calls).toHaveLength(0)
    expect(
      screen.getByText('Discard this snapshot permanently?'),
    ).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Keep' }))
    expect(transport.calls).toHaveLength(0)

    await user.click(screen.getAllByRole('button', { name: 'Discard' })[0]!)
    // The first row is now in its confirm state, so its own Discard is the one
    // that actually deletes the snapshot.
    await user.click(screen.getAllByRole('button', { name: 'Discard' })[0]!)

    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect(transport.calls[0]).toEqual({
      method: 'session.discard_recovery',
      params: { autosave_path: '/state/autosave/1.fwf' },
    })
  })

  it('closes once the last snapshot is dealt with', async () => {
    const user = userEvent.setup()
    setup()

    await user.click(screen.getAllByRole('button', { name: 'Recover' })[0]!)
    await waitFor(() =>
      expect(screen.getAllByRole('button', { name: 'Recover' })).toHaveLength(
        1,
      ),
    )
    await user.click(screen.getAllByRole('button', { name: 'Recover' })[0]!)

    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
  })
})

describe('checkForRecovery', () => {
  it('opens the dialog only when the kernel lists a snapshot', async () => {
    setTransport(createMockTransport({ 'session.recover_list': () => [] }))
    await checkForRecovery()
    expect(useUiStore.getState().dialog).toBeNull()

    setTransport(createMockTransport({ 'session.recover_list': () => ENTRIES }))
    await checkForRecovery()
    expect(useUiStore.getState().dialog).toEqual({
      kind: 'recovery',
      entries: ENTRIES,
    })
  })

  it('stays quiet when the kernel cannot list snapshots', async () => {
    setTransport(
      createMockTransport({
        'session.recover_list': () => {
          throw new Error('no recovery directory')
        },
      }),
    )

    await checkForRecovery()

    expect(useUiStore.getState().dialog).toBeNull()
    expect(useUiStore.getState().toasts).toEqual([])
  })
})
