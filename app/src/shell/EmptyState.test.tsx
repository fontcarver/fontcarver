import { act, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { useKeymapStore } from '../stores/keymapStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { EmptyState } from './EmptyState'
import { registerCommand, registerCommands, resetCommands } from './commands'
import { useContextHelpStore } from './contextHelp'
import { shellCommands } from './shellCommands'
import { formatShortcut } from './shortcuts'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'

const sessionActions = {
  open: useSessionStore.getState().open,
  openDialog: useSessionStore.getState().openDialog,
}

beforeEach(() => {
  resetCommands()
  registerCommands(shellCommands())
  registerWorkspaceTabCommands()
  useSessionStore.setState(initialSessionState())
  useUiStore.setState(initialUiState())
  useKeymapStore.setState({ open: false, overrides: {} })
  useContextHelpStore.getState().close()
})

afterEach(() => {
  resetCommands()
  useSessionStore.setState(sessionActions)
  vi.restoreAllMocks()
})

it('opens each existing discovery destination only after activation', async () => {
  render(<EmptyState />)
  expect(useContextHelpStore.getState().context).toBeNull()
  expect(useUiStore.getState().paletteOpen).toBe(false)
  expect(useKeymapStore.getState().open).toBe(false)
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Getting started' }))
  expect(useContextHelpStore.getState().context?.topic).toBe('welcome')
  await user.click(screen.getByRole('button', { name: /Command Palette/ }))
  expect(useUiStore.getState().paletteOpen).toBe(true)
  await user.click(screen.getByRole('button', { name: 'Keyboard Shortcuts' }))
  expect(useKeymapStore.getState().open).toBe(true)
})

it.each(['Linux', 'MacIntel'])(
  'updates effective shortcut hints on %s without remounting',
  (platform) => {
    vi.spyOn(navigator, 'platform', 'get').mockReturnValue(platform)
    render(<EmptyState />)
    const palette = screen.getByRole('button', { name: /Command Palette/ })
    expect(
      within(palette).getByText(formatShortcut('Mod+Shift+P')),
    ).toBeVisible()
    expect(within(palette).getByText(formatShortcut('Mod+K'))).toBeVisible()
    act(() =>
      useKeymapStore.setState({
        overrides: {
          'palette.open': 'Alt+U',
          'palette.quickOpen': null,
          'help.shortcuts': 'Mod+Alt+H',
        },
      }),
    )
    expect(within(palette).getByText(formatShortcut('Alt+U'))).toBeVisible()
    expect(palette).not.toHaveTextContent(formatShortcut('Mod+Shift+P'))
    expect(palette).not.toHaveTextContent(formatShortcut('Mod+K'))
    expect(
      screen.getByRole('button', { name: /Keyboard Shortcuts/ }),
    ).toHaveTextContent(formatShortcut('Mod+Alt+H'))
    act(() =>
      useKeymapStore.setState({
        overrides: {
          'palette.open': null,
          'palette.quickOpen': null,
          'help.shortcuts': null,
        },
      }),
    )
    expect(palette.querySelector('kbd')).toBeNull()
    expect(
      screen
        .getByRole('button', { name: 'Keyboard Shortcuts' })
        .querySelector('kbd'),
    ).toBeNull()
  },
)

it('reflects late command registration and avoids duplicate equivalent hints', () => {
  resetCommands()
  render(<EmptyState />)
  const palette = screen.getByRole('button', { name: /Command Palette/ })
  expect(palette.querySelector('kbd')).toBeNull()
  act(() => registerCommands(shellCommands()))
  expect(palette).toHaveTextContent(formatShortcut('Mod+Shift+P'))
  act(() => {
    registerWorkspaceTabCommands()
    useKeymapStore.setState({
      overrides: { 'palette.quickOpen': 'Mod+Shift+P' },
    })
  })
  expect(palette.querySelectorAll('kbd')).toHaveLength(1)
})

it('keeps New, Open and recent-file routing and keyboard access to all three rows', async () => {
  const open = vi.fn(async () => null)
  const openDialog = vi.fn(async () => undefined)
  const create = vi.fn()
  useSessionStore.setState({ recent: ['/fonts/Example.fcf'], open, openDialog })
  registerCommand({ id: 'file.new', title: 'cmd.file.new', run: create })
  render(<EmptyState />)
  const user = userEvent.setup()
  const buttons = screen.getAllByRole('button')
  expect(buttons).toHaveLength(6)
  for (const button of buttons) {
    await user.tab()
    expect(button).toHaveFocus()
  }
  await user.keyboard('{Enter}')
  expect(useKeymapStore.getState().open).toBe(true)
  await user.click(screen.getByRole('button', { name: /Open/ }))
  expect(openDialog).toHaveBeenCalledOnce()
  await user.click(screen.getByRole('button', { name: /New/ }))
  expect(create).toHaveBeenCalledOnce()
  await user.click(screen.getByRole('button', { name: '/fonts/Example.fcf' }))
  expect(open).toHaveBeenCalledWith('/fonts/Example.fcf')
})
