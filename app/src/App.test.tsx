import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from './api'
import { App } from './App'
import { resetCommands } from './shell/commands'
import { initialDocumentState, useDocumentStore } from './stores/documentStore'
import {
  DEFAULT_PREFS,
  initialPrefsState,
  usePrefsStore,
} from './stores/prefsStore'
import { initialSessionState, useSessionStore } from './stores/sessionStore'
import { forgetFontInfo } from './stores/fontInfoCache'
import { initialUiState, useUiStore } from './stores/uiStore'
import { makeFontInfoView } from './test/fontFixtures'
import { createMockTransport, makeSummary } from './test/mockTransport'

function emptySession(language = DEFAULT_PREFS.language) {
  return createMockTransport({
    'session.list': () => [],
    'session.recent': () => ['/fonts/DejaVuSans.ttf'],
    'session.recover_list': () => [],
    'prefs.get': () => ({ ...DEFAULT_PREFS, language }),
    'prefs.set': ({ prefs }) => prefs,
  })
}

beforeEach(() => {
  localStorage.clear()
  forgetFontInfo(1)
  document.documentElement.removeAttribute('data-theme')
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  usePrefsStore.setState(initialPrefsState())
})

afterEach(() => {
  setTransport(undefined)
  resetCommands()
})

describe('App shell', () => {
  it('keeps the document language aligned with loaded and changed preferences', async () => {
    setTransport(emptySession('zh-CN'))
    render(<App />)

    await screen.findByRole('menuitem', { name: '文件' })
    expect(document.documentElement.lang).toBe('zh-CN')

    act(() => useUiStore.getState().setLanguage('en'))
    await screen.findByRole('menuitem', { name: 'File' })
    expect(document.documentElement.lang).toBe('en')
  })

  it('renders every specified menu from the command registry', async () => {
    setTransport(emptySession())
    render(<App />)

    const menubar = await screen.findByRole('menubar')
    const menus = [
      'File',
      'Edit',
      'Glyph',
      'Path',
      'Layer',
      'Font',
      'Features',
      'Bitmap',
      'View',
      'Window',
      'Help',
    ]
    for (const label of menus)
      expect(
        within(menubar).getByRole('menuitem', { name: label }),
      ).toBeInTheDocument()
  })

  it('lists bitmap editing with a disabled reason when no document is open', async () => {
    const user = userEvent.setup()
    setTransport(emptySession())
    render(<App />)
    const menubar = await screen.findByRole('menubar')

    // Bitmap commands are listed but require an open document.
    await user.click(within(menubar).getByRole('menuitem', { name: 'Bitmap' }))

    const item = await screen.findByRole('menuitem', {
      name: /^Bitmap editor/,
    })
    expect(item).toBeDisabled()
  })

  it('keeps unavailable commands listed in the menu with a reason', async () => {
    const user = userEvent.setup()
    setTransport(emptySession())
    render(<App />)
    const menubar = await screen.findByRole('menubar')

    await user.click(within(menubar).getByRole('menuitem', { name: 'File' }))

    const menu = screen.getByRole('menu', { name: 'File' })
    const saveAs = within(menu).getByRole('menuitem', { name: /Save As/ })
    expect(saveAs).toBeDisabled()
    expect(saveAs).toHaveAttribute('title', 'No font is open')
    expect(within(menu).getAllByText('No font is open').length).toBeGreaterThan(
      0,
    )
  })

  it('follows a system theme change while the preference is system', async () => {
    const listeners = new Set<() => void>()
    const query = {
      matches: false,
      addEventListener: (_type: string, listener: () => void) =>
        listeners.add(listener),
      removeEventListener: (_type: string, listener: () => void) =>
        listeners.delete(listener),
    }
    vi.stubGlobal('matchMedia', () => query)
    setTransport(emptySession())
    useUiStore.setState({ theme: 'system' })
    render(<App />)
    await screen.findByRole('menubar')
    expect(document.documentElement.dataset.theme).toBe('light')

    query.matches = true
    for (const listener of [...listeners]) listener()

    await waitFor(() =>
      expect(document.documentElement.dataset.theme).toBe('dark'),
    )
    vi.unstubAllGlobals()
  })

  it('shows the empty state with recent files while no document is open', async () => {
    setTransport(emptySession())
    render(<App />)

    expect(
      screen.getByRole('heading', { name: 'FontCarver' }),
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
        'glyphs.list': () => ({ total: 6253, items: [] }),
      }),
    )
    render(<App />)

    expect(
      await screen.findByRole('tab', { name: 'DejaVuSans.ttf' }),
    ).toBeInTheDocument()
    expect(screen.getByRole('listbox', { name: 'Glyphs' })).toBeInTheDocument()
    expect(await screen.findByText('Letter')).toBeInTheDocument()
    expect(screen.getAllByText('6,253 glyphs').length).toBeGreaterThan(0)
    // Both counts are always on the status bar, selection included.
    expect(
      within(screen.getByRole('contentinfo')).getByText('0 selected'),
    ).toBeInTheDocument()
  })

  it('makes the sidebar and inspector follow the active tab kind', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'session.list': () => [makeSummary()],
        'session.recent': () => [],
        'doc.sidebar': () => ({
          categories: [['Letter', 5000] as [string, number]],
          scripts: [],
          blocks: [],
          unencoded: 0,
          total: 6253,
          color_labels: [],
        }),
        'glyphs.list': () => ({ total: 6253, items: [] }),
        'doc.font_info': () => makeFontInfoView(),
      }),
    )
    render(<App />)
    await screen.findByRole('tab', { name: 'DejaVuSans.ttf' })
    expect(await screen.findByText('Letter')).toBeInTheDocument()
    expect(screen.getAllByLabelText('Cell size')).toHaveLength(2)

    await user.keyboard('{Control>}i{/Control}')
    await screen.findByRole('tab', { name: 'Font Info' })

    expect(screen.queryByText('Letter')).toBeNull()
    expect(screen.queryAllByLabelText('Cell size')).toHaveLength(0)
    expect(
      screen.getByRole('button', { name: 'CID mapping' }),
    ).toBeInTheDocument()
    // The Font Info tab itself renders the read-only field list.
    expect(
      await screen.findByRole('searchbox', { name: 'Search fields' }),
    ).toBeInTheDocument()
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
        'glyphs.list': () => ({ total: 1, items: [] }),
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
