import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useKeymapStore } from '../stores/keymapStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { registerCommands, resetCommands } from './commands'
import { formatShortcut } from './shortcuts'
import { ShortcutSettingsDialog } from './ShortcutSettingsDialog'

/** `Mod+S` as this platform spells it, which is what the editor shows. */
const MOD_S = formatShortcut('Mod+S')

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

it('marks the conflicting row inline, not only the summary', async () => {
  const user = userEvent.setup()
  render(<ShortcutSettingsDialog />)
  const binding = screen.getByLabelText('Shortcut for Open Font…')
  await user.clear(binding)
  await user.type(binding, 'Mod+S')
  expect(binding).toHaveAttribute('aria-invalid', 'true')
  const other = screen.getByLabelText('Shortcut for Save')
  expect(other).toHaveAttribute('aria-invalid', 'true')
  expect(
    screen.getByText(`${MOD_S} is also bound to Save.`, { exact: false }),
  ).toBeVisible()
})

it('applies a conflict only once it is explicitly allowed, and persists that', async () => {
  const user = userEvent.setup()
  render(<ShortcutSettingsDialog />)
  const binding = screen.getByLabelText('Shortcut for Open Font…')
  await user.clear(binding)
  await user.type(binding, 'Mod+S')
  expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()

  await user.click(
    screen.getByRole('button', { name: `Allow ${MOD_S} to be shared` }),
  )
  expect(screen.getByLabelText('Shortcut for Open Font…')).toHaveAttribute(
    'aria-invalid',
    'false',
  )
  expect(screen.getByRole('button', { name: 'Apply' })).toBeEnabled()
  await user.click(screen.getByRole('button', { name: 'Apply' }))
  expect(useKeymapStore.getState().overrides).toEqual({
    'file.open': 'Mod+S',
  })
  expect(useKeymapStore.getState().allowConflicts).toEqual(['Mod+S'])
  expect(
    JSON.parse(localStorage.getItem('fontcarver.keymap.v1') ?? '{}'),
  ).toEqual({
    version: 1,
    bindings: { 'file.open': 'Mod+S' },
    allowConflicts: ['Mod+S'],
  })
})

it('blocks again when an allowed conflict is withdrawn', async () => {
  const user = userEvent.setup()
  useKeymapStore.setState({
    overrides: { 'file.open': 'Mod+S' },
    allowConflicts: ['Mod+S'],
    open: true,
  })
  render(<ShortcutSettingsDialog />)
  expect(screen.getByRole('button', { name: 'Apply' })).toBeEnabled()
  await user.click(
    screen.getByRole('button', { name: `Stop sharing ${MOD_S}` }),
  )
  expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()
})

it('does not keep an acceptance whose conflict was resolved', async () => {
  const user = userEvent.setup()
  useKeymapStore.setState({
    overrides: { 'file.open': 'Mod+S' },
    allowConflicts: ['Mod+S'],
    open: true,
  })
  render(<ShortcutSettingsDialog />)
  const binding = screen.getByLabelText('Shortcut for Open Font…')
  await user.clear(binding)
  await user.type(binding, 'Mod+K')
  await user.click(screen.getByRole('button', { name: 'Apply' }))
  expect(useKeymapStore.getState().allowConflicts).toEqual([])
  expect(
    JSON.parse(localStorage.getItem('fontcarver.keymap.v1') ?? '{}'),
  ).toEqual({ version: 1, bindings: { 'file.open': 'Mod+K' } })
})

it('shows a conflicting row even when the search matches many actions', async () => {
  const user = userEvent.setup()
  // More filler actions than the editor's row limit, registered after the two
  // real ones so the unsorted list would push both conflicts out of sight.
  registerCommands(
    Array.from({ length: 120 }, (_, index) => ({
      id: `filler.${index}`,
      title: 'cmd.edit.selectAll',
      run: vi.fn(),
    })),
  )
  render(<ShortcutSettingsDialog />)
  const binding = document.getElementById('binding-file.open')
  expect(binding).not.toBeNull()
  await user.clear(binding as HTMLElement)
  await user.type(binding as HTMLElement, 'Mod+S')
  expect(document.getElementById('conflict-file.open')).not.toBeNull()
  expect(document.getElementById('conflict-file.save')).not.toBeNull()
  expect(
    screen.getAllByText(`${MOD_S} is also bound to`, { exact: false }),
  ).toHaveLength(2)
})
