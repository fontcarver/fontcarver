import { i18n } from '../i18n'
import {
  activeDocId,
  selectActiveTab,
  useUiStore,
  type Tab,
} from '../stores/uiStore'
import { registerCommands, type Command } from './commands'
import { cycleTab, selectTab } from './tabActions'

/**
 * The tab a numbered shortcut selects.
 *
 * Position 0 is the document's pinned Overview. The rest address its editing
 * tabs in strip order — every kind of them, because the strip the user is
 * counting along holds Glyph, Font Info, Features and Bitmap tabs alike.
 */
function tabAt(index: number): Tab | undefined {
  const doc = activeDocId()
  const tabs = useUiStore.getState().tabs.filter((tab) => tab.doc === doc)
  return index === 0
    ? tabs.find((tab) => tab.kind === 'font')
    : tabs.filter((tab) => tab.kind !== 'font')[index - 1]
}

/** The active Edit tab, which is the one that owns a preview strip. */
function previewTab(): Tab | undefined {
  const tab = selectActiveTab(useUiStore.getState())
  return tab?.kind === 'glyph' ? tab : undefined
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
    {
      id: 'view.previewCollapse',
      title: 'cmd.view.previewCollapse',
      menu: 'view',
      group: 'panels',
      shortcut: 'Mod+Alt+P',
      detail: () =>
        previewTab()?.textState?.previewCollapsed === true
          ? i18n.t('cmd.view.previewCollapsed')
          : undefined,
      // `run` needs the tab's text state to toggle, so `when` requires it too:
      // a command that is offered as available and then does nothing is worse
      // than one that says it is unavailable.
      when: () =>
        previewTab()?.textState !== undefined &&
        useUiStore.getState().textPreviewVisible,
      disabledReason: 'cmd.reason.noPreview',
      run: () => {
        const tab = previewTab()
        if (!tab?.textState) return
        useUiStore.getState().updateTab(tab.id, {
          textState: {
            ...tab.textState,
            previewCollapsed: !tab.textState.previewCollapsed,
          },
        })
      },
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
