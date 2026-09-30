import { getTransport } from '../api'
import { i18n } from '../i18n'
import { useKeymapStore } from '../stores/keymapStore'
import { useWorkspaceStore } from '../stores/workspaceStore'
import { useDocumentStore } from '../stores/documentStore'
import { activeDocId as activeDoc, useUiStore } from '../stores/uiStore'
import type { Command } from './commands'
import { closeTab, cycleTab } from './tabActions'
import {
  detachActiveTab,
  detachPanel,
  openDocumentWindow,
} from './windowActions'

const APP_VERSION = '0.1.0'
const hasDocument = () => activeDoc() !== null

const capitalised = (value: string) =>
  `${value[0]?.toUpperCase() ?? ''}${value.slice(1)}`

/** Whether the active tab is an Edit tab, which is what owns a preview strip. */
function hasPreview(): boolean {
  const ui = useUiStore.getState()
  return ui.tabs.find((tab) => tab.id === ui.activeTab)?.kind === 'glyph'
}

function textCanvasHasFocus(): boolean {
  const ui = useUiStore.getState()
  return (
    Boolean(ui.tabs.find((tab) => tab.id === ui.activeTab)?.textState) &&
    globalThis.document?.activeElement instanceof HTMLCanvasElement
  )
}

/** Navigation, workspace and help actions shared by the menu and palette. */
export function shellCommands(): Command[] {
  return [
    {
      id: 'palette.open',
      title: 'cmd.palette.open',
      menu: 'view',
      group: 'palette',
      shortcut: 'Mod+Shift+P',
      allowInInput: true,
      run: () => useUiStore.getState().setPaletteOpen(true),
    },
    {
      id: 'window.nextTab',
      title: 'cmd.window.nextTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Alt+ArrowRight',
      when: () =>
        useUiStore.getState().tabs.length > 1 && !textCanvasHasFocus(),
      disabledReason: 'cmd.reason.oneTab',
      run: () => cycleTab(1),
    },
    {
      id: 'window.previousTab',
      title: 'cmd.window.previousTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Alt+ArrowLeft',
      when: () =>
        useUiStore.getState().tabs.length > 1 && !textCanvasHasFocus(),
      disabledReason: 'cmd.reason.oneTab',
      run: () => cycleTab(-1),
    },
    {
      id: 'window.closeTab',
      title: 'cmd.window.closeTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Shift+W',
      // The Overview is pinned: it stands for the document, so closing it is
      // File ▸ Close Font, not Close Tab.
      when: () => {
        const ui = useUiStore.getState()
        const tab = ui.tabs.find((candidate) => candidate.id === ui.activeTab)
        return tab !== undefined && tab.kind !== 'font'
      },
      disabledReason: 'cmd.reason.pinnedOverview',
      run: () => {
        const ui = useUiStore.getState()
        const tab = ui.tabs.find((candidate) => candidate.id === ui.activeTab)
        if (tab && tab.kind !== 'font') closeTab(tab.id)
      },
    },
    {
      id: 'window.detachTab',
      title: 'shellWindow.detachActive',
      menu: 'window',
      group: 'windows',
      when: () =>
        useUiStore
          .getState()
          .tabs.some(
            (tab) =>
              tab.id === useUiStore.getState().activeTab && tab.kind !== 'font',
          ),
      disabledReason: 'cmd.reason.noTab',
      run: detachActiveTab,
    },
    ...(['sidebar', 'inspector', 'preview'] as const).map((panel): Command => ({
      id: `window.detach${capitalised(panel)}`,
      title: `shellWindow.detach${capitalised(panel)}`,
      menu: 'window',
      group: 'windows',
      // Only an Edit tab owns a preview strip to tear off.
      when: panel === 'preview' ? hasPreview : hasDocument,
      disabledReason:
        panel === 'preview' ? 'cmd.reason.noGlyphTab' : 'cmd.reason.noDocument',
      run: () => detachPanel(panel),
    })),
    {
      id: 'window.document',
      title: 'shellWindow.document',
      menu: 'window',
      group: 'windows',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => openDocumentWindow(activeDoc()),
    },
    {
      id: 'workspace.manage',
      title: 'workspace.manage',
      menu: 'window',
      group: 'workspace',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => useWorkspaceStore.getState().setDialogOpen(true),
    },
    {
      id: 'glyph.renameBatch',
      title: 'dialog.batchRename.title',
      menu: 'glyph',
      group: 'glyph',
      when: () => {
        const doc = activeDoc()
        return (
          doc !== null &&
          (useDocumentStore.getState().byDoc[doc]?.selection.size ?? 0) > 0
        )
      },
      disabledReason: 'cmd.reason.noSelection',
      run: () => {
        const doc = activeDoc()
        if (doc !== null)
          useUiStore.getState().setDialog({
            kind: 'batchRename',
            doc,
            glyphs: [
              ...(useDocumentStore.getState().byDoc[doc]?.selection ?? []),
            ],
          })
      },
    },
    {
      id: 'help.shortcuts',
      title: 'cmd.help.shortcuts',
      menu: 'help',
      group: 'help',
      run: () => useKeymapStore.getState().setOpen(true),
    },
    {
      id: 'help.about',
      title: 'cmd.help.about',
      menu: 'help',
      group: 'help',
      run: () => {
        useUiStore.getState().pushToast({
          kind: 'info',
          message: i18n.t('toast.about', {
            version: APP_VERSION,
            kind: getTransport().kind,
          }),
        })
      },
    },
  ]
}
