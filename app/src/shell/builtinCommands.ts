import { call, getTransport, type DocId, type GlyphId } from '../api'
import { i18n } from '../i18n'
import { useDocumentStore } from '../stores/documentStore'
import { activeSummary, useSessionStore } from '../stores/sessionStore'
import {
  activeDocId,
  useUiStore,
  type ThemePreference,
} from '../stores/uiStore'

import { registerFontViewCommands } from '../views/font/fontCommands'
import { glyphName } from '../views/font/glyphNames'
import {
  registerGlyphViewCommands,
  zoomActiveGlyphView,
} from '../views/glyph/glyphCommands'

import { registerCommands, type Command } from './commands'
import { closeTab, openFontInfoTab, openGlyphTab } from './tabActions'

const APP_VERSION = '0.1.0'

function activeDoc(): DocId | null {
  return activeDocId()
}

function hasDocument(): boolean {
  return activeDoc() !== null
}

/**
 * The glyph a glyph-scoped command acts on: the grid's focus cell, which the
 * pointer, the keyboard and the context menu all move, falling back to the
 * first member of the selection.
 */
function targetGlyph(): GlyphId | undefined {
  const doc = activeDoc()
  if (doc === null) return undefined
  const view = useDocumentStore.getState().byDoc[doc]
  if (!view) return undefined
  if (view.focus !== null) return view.focus
  return [...view.selection][0]
}

function themeCommand(theme: ThemePreference): Command {
  return {
    id: `view.theme.${theme}`,
    title: `cmd.view.theme.${theme}`,
    menu: 'view',
    group: 'theme',
    when: () => useUiStore.getState().theme !== theme,
    disabledReason: 'cmd.reason.themeActive',
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
  registerFontViewCommands()
  registerGlyphViewCommands()
  registerCommands([
    {
      id: 'file.new',
      title: 'cmd.file.new',
      menu: 'file',
      group: 'open',
      shortcut: 'Mod+N',
      run: () =>
        useSessionStore
          .getState()
          .createDocument()
          .then(() => undefined),
    },
    {
      id: 'file.open',
      title: 'cmd.file.open',
      menu: 'file',
      group: 'open',
      shortcut: 'Mod+O',
      run: () => useSessionStore.getState().openDialog(),
    },
    {
      id: 'file.openRecent',
      title: 'cmd.file.openRecent',
      menu: 'file',
      group: 'open',
      when: () => useSessionStore.getState().recent.length > 0,
      disabledReason: 'cmd.reason.noRecent',
      run: () => useUiStore.getState().setPathPrompt({ mode: 'open' }),
    },
    {
      id: 'file.save',
      title: 'cmd.file.save',
      menu: 'file',
      group: 'save',
      shortcut: 'Mod+S',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDoc()
        return doc === null
          ? undefined
          : useSessionStore
              .getState()
              .save(doc)
              .then(() => undefined)
      },
    },
    {
      id: 'file.saveAs',
      title: 'cmd.file.saveAs',
      menu: 'file',
      group: 'save',
      shortcut: 'Mod+Shift+S',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDoc()
        return doc === null
          ? undefined
          : useSessionStore.getState().requestSaveAs(doc)
      },
    },
    {
      id: 'file.fontInfo',
      title: 'cmd.file.fontInfo',
      menu: 'file',
      group: 'info',
      shortcut: 'Mod+I',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
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
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDoc()
        return doc === null
          ? undefined
          : useSessionStore.getState().requestClose(doc)
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
      disabledReason: 'cmd.reason.nothingToUndo',
      run: () => runHistory('doc.undo'),
    },
    {
      id: 'edit.redo',
      title: 'cmd.edit.redo',
      menu: 'edit',
      group: 'history',
      shortcut: 'Mod+Shift+Z',
      detail: () => activeSummary()?.redo_label ?? undefined,
      when: () => activeSummary()?.redo_label != null,
      disabledReason: 'cmd.reason.nothingToRedo',
      run: () => runHistory('doc.redo'),
    },
    {
      id: 'glyph.open',
      title: 'cmd.glyph.open',
      menu: 'glyph',
      group: 'glyph',
      when: () => targetGlyph() !== undefined,
      disabledReason: 'cmd.reason.noSelection',
      run: () => {
        const doc = activeDoc()
        const gid = targetGlyph()
        if (doc !== null && gid !== undefined)
          openGlyphTab(doc, gid, glyphName(doc, gid))
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
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        if (zoomActiveGlyphView(1)) return
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
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        if (zoomActiveGlyphView(-1)) return
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
      disabledReason: 'cmd.reason.oneTab',
      run: () => useUiStore.getState().cycleTab(1),
    },
    {
      id: 'window.previousTab',
      title: 'cmd.window.previousTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Alt+ArrowLeft',
      when: () => useUiStore.getState().tabs.length > 1,
      disabledReason: 'cmd.reason.oneTab',
      run: () => useUiStore.getState().cycleTab(-1),
    },
    {
      id: 'window.closeTab',
      title: 'cmd.window.closeTab',
      menu: 'window',
      group: 'tabs',
      shortcut: 'Mod+Shift+W',
      when: () => useUiStore.getState().activeTab !== null,
      disabledReason: 'cmd.reason.noTab',
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
        useUiStore.getState().pushToast({
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
