import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../api'
import { App } from '../App'
import { resetCommands } from '../shell/commands'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import {
  DEFAULT_PREFS,
  initialPrefsState,
  usePrefsStore,
} from '../stores/prefsStore'
import { initialProgressState, useProgressStore } from '../stores/progressStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

function shell(summary = makeSummary()) {
  const transport = createMockTransport({
    'session.list': () => [summary],
    'session.recent': () => [],
    'session.recover_list': () => [],
    'prefs.get': () => DEFAULT_PREFS,
    'prefs.set': ({ prefs }) => prefs,
    'doc.sidebar': () => ({
      categories: [],
      scripts: [],
      blocks: [],
      unencoded: 0,
      total: 0,
      color_labels: [],
    }),
    'glyphs.list': () => ({ total: 0, items: [] }),
    'glyphs.add': () => ({ added: [], skipped: [] }),
    'doc.find_problems': ({ doc }) => ({
      doc,
      generation: 0,
      issues: [],
      total: 0,
      truncated: false,
      has_errors: false,
    }),
  })
  setTransport(transport)
  render(<App />)
  return transport
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  usePrefsStore.setState(initialPrefsState())
  useProgressStore.setState(initialProgressState())
})

afterEach(() => {
  setTransport(undefined)
  resetCommands()
})

describe('dialog wiring', () => {
  it('opens New Font with its shortcut', async () => {
    const user = userEvent.setup()
    shell()

    await user.keyboard('{Control>}n{/Control}')

    expect(
      await screen.findByRole('dialog', { name: 'New Font' }),
    ).toBeInTheDocument()
  })

  it('opens Add Glyphs, Export and Preferences from their shortcuts', async () => {
    const user = userEvent.setup()
    shell()
    await screen.findByRole('tab', { name: 'DejaVuSans.ttf' })

    await user.keyboard('{Control>}{Shift>}G{/Shift}{/Control}')
    expect(
      await screen.findByRole('dialog', { name: 'Add Glyphs' }),
    ).toBeInTheDocument()
    await user.keyboard('{Escape}')

    await user.keyboard('{Control>}e{/Control}')
    expect(
      await screen.findByRole('dialog', { name: 'Export' }),
    ).toBeInTheDocument()
    await user.keyboard('{Escape}')

    await user.keyboard('{Control>},{/Control}')
    expect(
      await screen.findByRole('dialog', { name: 'Preferences' }),
    ).toBeInTheDocument()
  })

  it('lists Export and Add Glyphs in their menus', async () => {
    const user = userEvent.setup()
    shell()
    await screen.findByRole('tab', { name: 'DejaVuSans.ttf' })

    await user.click(screen.getByRole('menuitem', { name: 'File' }))
    await user.click(screen.getByRole('menuitem', { name: /^Export/ }))

    expect(
      await screen.findByRole('dialog', { name: 'Export' }),
    ).toBeInTheDocument()
  })

  it('draws the busy line while an operation runs', async () => {
    const transport = shell()
    await screen.findByRole('tab', { name: 'DejaVuSans.ttf' })

    transport.emit({ kind: 'Progress', doc: 1, stage: 'compile', percent: 40 })

    const line = await screen.findByRole('progressbar')
    expect(line).toHaveAttribute('aria-valuenow', '40')

    transport.emit({ kind: 'Progress', doc: 1, stage: 'write', percent: 100 })
    await waitFor(() =>
      expect(screen.queryByRole('progressbar')).not.toBeInTheDocument(),
    )
  })

  it('opens Find problems from the Font menu and command palette', async () => {
    const user = userEvent.setup()
    shell()
    await screen.findByRole('tab', { name: 'DejaVuSans.ttf' })
    await user.click(screen.getByRole('menuitem', { name: 'Font' }))
    await user.click(screen.getByRole('menuitem', { name: 'Find problems…' }))
    expect(
      await screen.findByRole('dialog', { name: 'Find problems' }),
    ).toBeVisible()
    expect(await screen.findByText('No problems found.')).toBeVisible()
    await user.keyboard('{Escape}')
    await user.keyboard('{Control>}{Shift>}P{/Shift}{/Control}')
    await user.type(
      screen.getByRole('combobox', { name: 'Commands' }),
      'Find problems',
    )
    await user.keyboard('{Enter}')
    expect(
      await screen.findByRole('dialog', { name: 'Find problems' }),
    ).toBeVisible()
  })

  it('warns that a recovered document still points at the original file', async () => {
    const user = userEvent.setup()
    shell(
      makeSummary({
        dirty: true,
        recovered_from: '/fonts/Workshop.fcf',
      }),
    )

    const banner = await screen.findByRole('status')
    expect(banner).toHaveTextContent('/fonts/Workshop.fcf')
    expect(banner).toHaveTextContent('saving overwrites the original')

    await user.click(screen.getByRole('button', { name: 'Dismiss banner' }))
    await waitFor(() =>
      expect(screen.queryByRole('status')).not.toBeInTheDocument(),
    )
  })
})
