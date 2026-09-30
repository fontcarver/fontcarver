import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useKeymapStore } from '../stores/keymapStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { registerCommands, resetCommands } from './commands'
import { ShortcutSettingsDialog } from './ShortcutSettingsDialog'

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useKeymapStore.setState({ overrides: {}, open: true })
  registerCommands([
    {
      id: 'file.open',
      title: 'cmd.file.open',
      shortcut: 'Mod+O',
      run: vi.fn(),
    },
    {
      id: 'file.save',
      title: 'cmd.file.save',
      shortcut: 'Mod+S',
      run: vi.fn(),
    },
  ])
})
afterEach(() => {
  resetCommands()
  useKeymapStore.setState({ overrides: {}, open: false })
  vi.restoreAllMocks()
})

it('stages changes and blocks a conflict until resolved', async () => {
  const user = userEvent.setup()
  render(<ShortcutSettingsDialog />)
  const binding = screen.getByLabelText('Shortcut for Open Font…')
  await user.clear(binding)
  await user.type(binding, 'Ctrl+S')
  expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()
  expect(screen.getByRole('alert')).toHaveTextContent('Save')
  expect(useKeymapStore.getState().overrides).toEqual({})
  await user.clear(binding)
  await user.type(binding, 'Mod+K')
  await user.click(screen.getByRole('button', { name: 'Apply' }))
  expect(useKeymapStore.getState().overrides).toEqual({ 'file.open': 'Mod+K' })
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
})

it('cancels draft changes without replacing the active keymap', async () => {
  const user = userEvent.setup()
  useKeymapStore.setState({ overrides: { 'file.open': 'Mod+K' } })
  render(<ShortcutSettingsDialog />)
  await user.click(screen.getByRole('button', { name: 'Restore Open Font…' }))
  await user.click(screen.getByRole('button', { name: 'Cancel' }))
  expect(useKeymapStore.getState().overrides).toEqual({ 'file.open': 'Mod+K' })
})

it('imports JSON as a reviewed draft and reports invalid imports', async () => {
  const user = userEvent.setup()
  render(<ShortcutSettingsDialog />)
  const input = screen.getByLabelText('Import keymap JSON')
  const file = new File(
    ['{"version":1,"bindings":{"file.open":"Mod+K"}}'],
    'keys.json',
    { type: 'application/json' },
  )
  Object.defineProperty(file, 'text', {
    value: async () => '{"version":1,"bindings":{"file.open":"Mod+K"}}',
  })
  await user.upload(input, file)
  expect(await screen.findByDisplayValue('Mod+K')).toBeInTheDocument()
  expect(useKeymapStore.getState().overrides).toEqual({})
  const invalid = new File(['{}'], 'bad.json', { type: 'application/json' })
  Object.defineProperty(invalid, 'text', { value: async () => '{}' })
  await user.upload(input, invalid)
  expect(await screen.findByRole('alert')).toHaveTextContent('Invalid keymap')
})

it('keeps the editor open when persistence fails', async () => {
  const user = userEvent.setup()
  render(<ShortcutSettingsDialog />)
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('quota')
  })
  await user.click(screen.getByRole('button', { name: 'Apply' }))
  expect(screen.getByRole('alert')).toHaveTextContent('Could not save')
  expect(screen.getByRole('dialog')).toBeInTheDocument()
  act(() => useKeymapStore.setState({ open: false }))
})
