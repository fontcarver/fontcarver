import { beforeEach, expect, it } from 'vitest'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { getCommand, resetCommands } from './commands'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  resetCommands()
  registerWorkspaceTabCommands()
})

it('selects Overview and numbered Edit tabs within the active document', async () => {
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'glyph', doc: 2, gid: 99 })
  const overview = ui.openTab({ kind: 'font', doc: 1 })
  const glyph = ui.openTab({ kind: 'glyph', doc: 1, gid: 3 })
  ui.openTab({ kind: 'fontinfo', doc: 1 })
  await getCommand('window.tab.1')?.run()
  expect(useUiStore.getState().activeTab).toBe(overview)
  await getCommand('window.tab.2')?.run()
  expect(useUiStore.getState().activeTab).toBe(glyph)
  expect(getCommand('window.tab.3')?.when?.()).toBe(false)
  expect(getCommand('window.cycleNext')?.shortcut).toBe('Ctrl+Tab')
  expect(getCommand('window.tab.2')?.shortcut).toBe('Mod+Alt+2')
})

it('opens the palette with the documented Mod+K shortcut', async () => {
  const command = getCommand('palette.quickOpen')
  expect(command?.shortcut).toBe('Mod+K')
  await command?.run()
  expect(useUiStore.getState().paletteOpen).toBe(true)
})
