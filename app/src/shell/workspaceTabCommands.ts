import { activeDocId, useUiStore } from '../stores/uiStore'
import { registerCommands, type Command } from './commands'
import { cycleTab, selectTab } from './tabActions'

function tabAt(index: number) {
  const doc = activeDocId()
  const tabs = useUiStore.getState().tabs.filter((tab) => tab.doc === doc)
  return index === 0
    ? tabs.find((tab) => tab.kind === 'font')
    : tabs.filter((tab) => tab.kind === 'glyph')[index - 1]
}

/** Registers direct Overview/Edit tab selection and conventional tab cycling. */
export function registerWorkspaceTabCommands(): void {
  registerCommands([
    {
      id: 'palette.quickOpen',
      title: 'cmd.palette.open',
      shortcut: 'Mod+K',
      allowInInput: true,
      run: () => useUiStore.getState().setPaletteOpen(true),
    },
    ...Array.from({ length: 9 }, (_, index): Command => ({
      id: `window.tab.${index + 1}`,
      title: index === 0 ? 'shellWindow.overview' : 'shellWindow.editTab',
      detail: () => (index === 0 ? undefined : String(index + 1)),
      shortcut: `Mod+Alt+${index + 1}`,
      allowInInput: true,
      when: () => tabAt(index) !== undefined,
      disabledReason: 'cmd.reason.noTab',
      run: async () => {
        const tab = tabAt(index)
        if (tab) await selectTab(tab.id)
      },
    })),
    ...([1, -1] as const).map((delta): Command => ({
      id: delta > 0 ? 'window.cycleNext' : 'window.cyclePrevious',
      title: delta > 0 ? 'cmd.window.nextTab' : 'cmd.window.previousTab',
      shortcut: delta > 0 ? 'Ctrl+Tab' : 'Ctrl+Shift+Tab',
      allowInInput: true,
      when: () => useUiStore.getState().tabs.length > 1,
      disabledReason: 'cmd.reason.oneTab',
      run: () => cycleTab(delta),
    })),
  ])
}
