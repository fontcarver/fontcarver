import { beforeEach, expect, it } from 'vitest'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import { getCommand, resetCommands } from './commands'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'
import { commandForEvent } from './shortcuts'

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
  const info = ui.openTab({ kind: 'fontinfo', doc: 1 })
  await getCommand('window.tab.1')?.run()
  expect(useUiStore.getState().activeTab).toBe(overview)
  await getCommand('window.tab.2')?.run()
  expect(useUiStore.getState().activeTab).toBe(glyph)
  // Every editing tab of the document is reachable by its position in the
  // strip, not only the Glyph ones.
  await getCommand('window.tab.3')?.run()
  expect(useUiStore.getState().activeTab).toBe(info)
  expect(getCommand('window.tab.4')?.when?.()).toBe(false)
  expect(getCommand('window.cycleNext')?.shortcut).toBe('Control+Tab')
  expect(getCommand('window.cyclePrevious')?.shortcut).toBe('Control+Shift+Tab')
  expect(getCommand('window.tab.2')?.shortcut).toBe('Mod+Alt+2')
})

it.each([false, true])(
  'cycles with physical Control while a field is focused on macOS=%s',
  (mac) => {
    const ui = useUiStore.getState()
    ui.openTab({ kind: 'font', doc: 1 })
    ui.openTab({ kind: 'glyph', doc: 1, gid: 3 })
    const input = document.createElement('input')
    document.body.append(input)
    input.focus()
    try {
      for (const shiftKey of [false, true]) {
        const event = new KeyboardEvent('keydown', {
          key: 'Tab',
          ctrlKey: true,
          shiftKey,
          bubbles: true,
        })
        input.dispatchEvent(event)
        expect(commandForEvent(event, mac)).toBe(
          shiftKey ? 'window.cyclePrevious' : 'window.cycleNext',
        )
      }
      expect(
        commandForEvent(
          new KeyboardEvent('keydown', { key: 'Tab', metaKey: true }),
          mac,
        ),
      ).toBeUndefined()
    } finally {
      input.remove()
    }
  },
)

it('keeps the Overview pinned first however its tabs were opened', () => {
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'glyph', doc: 1, gid: 3 })
  ui.openTab({ kind: 'features', doc: 1 })
  const overview = ui.openTab({ kind: 'font', doc: 1 })
  const other = ui.openTab({ kind: 'font', doc: 2 })
  ui.openTab({ kind: 'glyph', doc: 2, gid: 4 })

  const tabs = useUiStore.getState().tabs
  expect(tabs[0]?.id).toBe(overview)
  expect(tabs.findIndex((tab) => tab.id === other)).toBe(3)
  expect(tabs.map((tab) => tab.kind)).toEqual([
    'font',
    'glyph',
    'features',
    'font',
    'glyph',
  ])
})

it('opens the palette with the documented Mod+K shortcut', async () => {
  const command = getCommand('palette.quickOpen')
  expect(command?.shortcut).toBe('Mod+K')
  await command?.run()
  expect(useUiStore.getState().paletteOpen).toBe(true)
})

it('collapses and expands the active Edit tab preview strip', async () => {
  const command = getCommand('view.previewCollapse')
  expect(command?.shortcut).toBe('Mod+Alt+P')
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'font', doc: 1 })
  // Without an Edit tab there is no strip, and the command says so.
  expect(command?.when?.()).toBe(false)

  const glyph = ui.openTab({ kind: 'glyph', doc: 1, gid: 3 })
  // An Edit tab that has not entered the Text workflow yet has no text state,
  // so there is nothing to collapse and the command says so rather than being
  // offered and doing nothing.
  expect(command?.when?.()).toBe(false)
  useUiStore.getState().updateTab(glyph, {
    textState: {
      text: 'A',
      activeIndex: 0,
      mode: 'edit',
      master: null,
      size: 72,
      features: [],
      direction: 'ltr',
      previewTheme: 'light',
      previewCollapsed: false,
    },
  })
  expect(command?.when?.()).toBe(true)

  await command?.run()
  const collapsed = () =>
    useUiStore.getState().tabs.find((tab) => tab.id === glyph)?.textState
      ?.previewCollapsed
  expect(collapsed()).toBe(true)
  await command?.run()
  expect(collapsed()).toBe(false)

  // A hidden strip has nothing to collapse; the visibility command owns that.
  useUiStore.getState().setTextPreviewVisible(false)
  expect(command?.when?.()).toBe(false)
  expect(glyph).toBe(tabId('glyph', 1, 3))
})
