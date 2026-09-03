import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from './api'
import { App } from './App'
import { resetCommands } from './shell/commands'
import { initialDocumentState, useDocumentStore } from './stores/documentStore'
import { initialSessionState, useSessionStore } from './stores/sessionStore'
import { initialUiState, useUiStore } from './stores/uiStore'
import { createMockTransport, makeSummary } from './test/mockTransport'

function emptySession() {
  return createMockTransport({
    'session.list': () => [],
    'session.recent': () => ['/fonts/DejaVuSans.ttf'],
  })
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
})

afterEach(() => {
  setTransport(undefined)
  resetCommands()
})

describe('App shell', () => {
  it('renders the menu bar built from the command registry', async () => {
    setTransport(emptySession())
    render(<App />)

    const menubar = await screen.findByRole('menubar')
    for (const label of ['File', 'Edit', 'Glyph', 'View', 'Window', 'Help'])
      expect(
        within(menubar).getByRole('menuitem', { name: label }),
      ).toBeInTheDocument()
  })

  it('shows the empty state with recent files while no document is open', async () => {
    setTransport(emptySession())
    render(<App />)

    expect(
      screen.getByRole('heading', { name: 'FontWorkshop' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Open Font…' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'New Font…' }),
    ).toBeInTheDocument()
    expect(
      await screen.findByRole('button', { name: '/fonts/DejaVuSans.ttf' }),
    ).toBeInTheDocument()
    expect(screen.getByText('No document open')).toBeInTheDocument()
  })

  it('opens the command palette with Mod+Shift+P and closes it with Escape', async () => {
    const user = userEvent.setup()
    setTransport(emptySession())
    render(<App />)
    await screen.findByRole('menubar')

    await user.keyboard('{Control>}{Shift>}p{/Shift}{/Control}')
    expect(
      await screen.findByRole('dialog', { name: 'Commands' }),
    ).toBeInTheDocument()

    await user.keyboard('{Escape}')
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'Commands' })).toBeNull(),
    )
  })

  it('switches the theme from the palette and writes data-theme', async () => {
    const user = userEvent.setup()
    setTransport(emptySession())
    render(<App />)
    await screen.findByRole('menubar')

    await user.keyboard('{Control>}{Shift>}p{/Shift}{/Control}')
    await user.type(await screen.findByRole('combobox'), 'dark theme')
    await user.keyboard('{Enter}')

    await waitFor(() =>
      expect(document.documentElement.dataset.theme).toBe('dark'),
    )
    expect(useUiStore.getState().theme).toBe('dark')
  })

  it('adopts a document already open in the kernel and renders its Font tab', async () => {
    setTransport(
      createMockTransport({
        'session.list': () => [makeSummary()],
        'session.recent': () => [],
        'doc.sidebar': () => ({
          categories: [['Letter', 5000]],
          scripts: [['Latn', 4000]],
          blocks: [],
          unencoded: 12,
          total: 6253,
          color_labels: [],
        }),
      }),
    )
    render(<App />)

    expect(
      await screen.findByRole('tab', { name: 'DejaVuSans.ttf' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: 'Font view' }),
    ).toBeInTheDocument()
    expect(await screen.findByText('Letter')).toBeInTheDocument()
    expect(screen.getByText('6,253 glyphs')).toBeInTheDocument()
  })

  it('toggles the inspector from the tab bar', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'session.list': () => [makeSummary()],
        'session.recent': () => [],
        'doc.sidebar': () => ({
          categories: [],
          scripts: [],
          blocks: [],
          unencoded: 0,
          total: 1,
          color_labels: [],
        }),
      }),
    )
    render(<App />)
    await screen.findByRole('tab', { name: 'DejaVuSans.ttf' })

    expect(screen.getByText('Inspector')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Toggle inspector' }))

    await waitFor(() => expect(screen.queryByText('Inspector')).toBeNull())
    expect(useUiStore.getState().panels.inspector).toBe(false)
  })
})
