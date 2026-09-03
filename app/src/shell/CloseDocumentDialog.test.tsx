import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { CloseDocumentDialog } from './CloseDocumentDialog'

function setup(summary = makeSummary({ dirty: true })) {
  const transport = createMockTransport({
    'session.save': () => makeSummary({ path: '/docs/a.fwf', dirty: false }),
    'session.close': () => null,
    'session.recent': () => [],
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().setClosePrompt({ doc: summary.doc })
  render(<CloseDocumentDialog />)
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

describe('CloseDocumentDialog', () => {
  it('offers Save, Don’t Save and Cancel for a dirty document', () => {
    setup()

    expect(
      screen.getByRole('dialog', { name: 'Unsaved changes' }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Don’t Save' }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument()
  })

  it('keeps the document when Cancel is chosen', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(useUiStore.getState().closePrompt).toBeNull()
    expect(transport.calls).toHaveLength(0)
    expect(useSessionStore.getState().docs).toHaveLength(1)
  })

  it('closes without saving when Don’t Save is chosen', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getByRole('button', { name: 'Don’t Save' }))

    await waitFor(() => expect(useSessionStore.getState().docs).toEqual([]))
    expect(transport.calls.map((call) => call.method)).toEqual([
      'session.close',
    ])
  })

  it('saves and then closes when Save is chosen', async () => {
    const user = userEvent.setup()
    const transport = setup(makeSummary({ path: '/docs/a.fwf', dirty: true }))

    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(useSessionStore.getState().docs).toEqual([]))
    const methods = transport.calls.map((call) => call.method)
    expect(methods).toContain('session.save')
    expect(methods.indexOf('session.close')).toBeGreaterThan(
      methods.indexOf('session.save'),
    )
  })

  it('asks for a path before saving a document that has none', async () => {
    const user = userEvent.setup()
    const transport = setup(makeSummary({ path: null, dirty: true }))

    await user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(useUiStore.getState().pathPrompt).toEqual({
        mode: 'saveAs',
        doc: 1,
        closeAfterSave: true,
      }),
    )
    expect(transport.calls).toHaveLength(0)
    expect(useSessionStore.getState().docs).toHaveLength(1)
  })
})
