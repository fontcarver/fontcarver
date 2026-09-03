import { call, getTransport, type DocId } from '../api'
import { i18n } from '../i18n'
import { useDocumentStore } from '../stores/documentStore'
import { selectActiveSummary, useSessionStore } from '../stores/sessionStore'
import { useUiStore, type ThemePreference } from '../stores/uiStore'

import { registerCommands, type Command } from './commands'
import { closeTab, openFontInfoTab, openGlyphTab } from './tabActions'

const APP_VERSION = '0.1.0'

function activeDoc(): DocId | null {
  return useSessionStore.getState().activeDoc
}

function hasDocument(): boolean {
  return activeDoc() !== null
}

function activeSummary() {
  return selectActiveSummary(useSessionStore.getState())
}

function themeCommand(theme: ThemePreference): Command {
  return {
    id: `view.theme.${theme}`,
    title: `cmd.view.theme.${theme}`,
    menu: 'view',
    group: 'theme',
    when: () => useUiStore.getState().theme !== theme,
    run: () => useUiStore.getState().setTheme(theme),
  }
}

async function runHistory(method: 'doc.undo' | 'doc.redo'): Promise<void> {
  const doc = activeDoc()
  if (doc === null) return
  await call(method, { doc })
  await useSessionStore.getState().refreshSummary(doc)
}

/** Registers every built-in command. Safe to call more than once. */
export function registerBuiltinCommands(): void {
  registerCommands([
    {
      id: 'file.new',
      title: 'cmd.file.new',
      menu: 'file',
      group: 'open',
      shortcut: 'Mod+N',
      run: () => void useSessionStore.getState().createDocument(),
    },
    {
      id: 'file.open',
      title: 'cmd.file.open',
      menu: 'file',
      group: 'open',
      shortcut: 'Mod+O',
      run: () => void useSessionStore.getState().openDialog(),
    },
    {
      id: 'file.openRecent',
      title: 'cmd.file.openRecent',
      menu: 'file',
      group: 'open',
      when: () => useSessionStore.getState().recent.length > 0,
      run: () => useUiStore.getState().setPathPrompt({ mode: 'open' }),
    },
    {
      id: 'file.save',
      title: 'cmd.file.save',
      menu: 'file',
      group: 'save',
      shortcut: 'Mod+S',
      when: hasDocument,
      run: () => {
        const doc = activeDoc()
        if (doc !== null) void useSessionStore.getState().save(doc)
      },
    },
    {
      id: 'file.saveAs',
      title: 'cmd.file.saveAs',
      menu: 'file',
      group: 'save',
      shortcut: 'Mod+Shift+S',
      when: hasDocument,
      run: () => {
        const doc = activeDoc()
        if (doc !== null) useSessionStore.getState().requestSaveAs(doc)
      },
    },
    {
      id: 'file.fontInfo',
      title: 'cmd.file.fontInfo',
      menu: 'file',
      group: 'info',
      shortcut: 'Mod+I',
      when: hasDocument,
      run: () => {
        const doc = activeDoc()
        if (doc !== null) openFontInfoTab(doc)
      },
    },
    {
      id: 'file.close',
      title: 'cmd.file.close',
      menu: 'file',
      group: 'close',
      shortcut: 'Mod+W',
      when: hasDocument,
      run: () => {
        const doc = activeDoc()
        if (doc !== null) void useSessionStore.getState().requestClose(doc)
      },
    },
    {
      id: 'edit.undo',
      title: 'cmd.edit.undo',
      menu: 'edit',
      group: 'history',
      shortcut: 'Mod+Z',
      detail: () => activeSummary()?.undo_label ?? undefined,
      when: () => activeSummary()?.undo_label != null,
      run: () => void runHistory('doc.undo'),
    },
    {
      id: 'edit.redo',
      title: 'cmd.edit.redo',
      menu: 'edit',
      group: 'history',
      shortcut: 'Mod+Shift+Z',
      detail: () => activeSummary()?.redo_label ?? undefined,
      when: () => activeSummary()?.redo_label != null,
      run: () => void runHistory('doc.redo'),
    },
    {
      id: 'glyph.open',
      title: 'cmd.glyph.open',
      menu: 'glyph',
      group: 'glyph',
      when: () => {
        const doc = activeDoc()
        if (doc === null) return false
        return (useDocumentStore.getState().byDoc[doc]?.selection.size ?? 0) > 0
      },
      run: () => {
        const doc = activeDoc()
        if (doc === null) return
        const selection = useDocumentStore.getState().byDoc[doc]?.selection
        const first = selection ? [...selection][0] : undefined
        if (first !== undefined) openGlyphTab(doc, first)
      },
    },
    {
      id: 'view.toggleSidebar',
      title: 'cmd.view.toggleSidebar',
      menu: 'view',
      group: 'panels',
      shortcut: 'Mod+1',
      run: () => useUiStore.getState().togglePanel('sidebar'),
    },
    {
      id: 'view.toggleInspector',
      title: 'cmd.view.toggleInspector',
      menu: 'view',
      group: 'panels',
      shortcut: 'Mod+2',
      run: () => useUiStore.getState().togglePanel('inspector'),
    },
    {
      id: 'view.zoomIn',
      title: 'cmd.view.zoomIn',
      menu: 'view',
      group: 'zoom',
      shortcut: 'Mod+=',
      when: hasDocument,
      run: () => {
        const doc = activeDoc()
        if (doc !== null) useDocumentStore.getState().stepCellSize(doc, 1)
      },
    },
    {
      id: 'view.zoomOut',
      title: 'cmd.view.zoomOut',
      menu: 'view',
      group: 'zoom',
      shortcut: 'Mod+-',
      when: hasDocument,
      run: () => {
        const doc = activeDoc()
        if (doc !== null) useDocumentStore.getState().stepCellSize(doc, -1)
      },
    },
    themeCommand('light'),
    themeCommand('dark'),
    themeCommand('system'),
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
      when: () => useUiStore.getState().tabs.length > 1,
      run: () => useUiStore.getState().cycleTab(1),
    },
    {
      id: 'window.previousTab',
      title: 'cmd.window.previousTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Alt+ArrowLeft',
      when: () => useUiStore.getState().tabs.length > 1,
      run: () => useUiStore.getState().cycleTab(-1),
    },
    {
      id: 'window.closeTab',
      title: 'cmd.window.closeTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Shift+W',
      when: () => useUiStore.getState().activeTab !== null,
      run: () => {
        const active = useUiStore.getState().activeTab
        if (active) closeTab(active)
      },
    },
    {
      id: 'help.shortcuts',
      title: 'cmd.help.shortcuts',
      menu: 'help',
      group: 'help',
      run: () => useUiStore.getState().setPaletteOpen(true),
    },
    {
      id: 'help.about',
      title: 'cmd.help.about',
      menu: 'help',
      group: 'help',
      run: () => {
        const { pushToast } = useUiStore.getState()
        pushToast({
          kind: 'info',
          message: i18n.t('toast.about', {
            version: APP_VERSION,
            kind: getTransport().kind,
          }),
        })
      },
    },
  ])
}
