import { registerCrossDocumentCommands } from '../views/crossdocument/crossDocumentCommands'
import { call, type DocId, type GlyphId } from '../api'
import { registerAutomationCommands } from '../automation/commands'
import { registerHelpCommands } from './contextHelp'
import { shellCommands } from './shellCommands'
import { withSettledDocument } from '../editor/documentPersistence'
import { hasPendingDocumentDraft } from '../editor/documentEditParticipants'
import { useDocumentStore } from '../stores/documentStore'
import { usePrefsStore } from '../stores/prefsStore'
import { activeSummary, useSessionStore } from '../stores/sessionStore'
import {
  activeDocId,
  useUiStore,
  type ThemePreference,
} from '../stores/uiStore'

import { SUPPORTED_LANGUAGES, type Language } from '../i18n'
import { registerFontViewCommands } from '../views/font/fontCommands'
import { glyphName } from '../views/font/glyphNames'
import {
  registerGlyphViewCommands,
  zoomActiveGlyphView,
} from '../views/glyph/glyphCommands'
import { registerBitmapCommands } from '../views/bitmap/bitmapCommands'
import { registerLayerCommands } from '../views/glyph/layerCommands'
import { registerMetricsCommands } from '../views/glyph/panels/metricsCommands'
import { registerPathCommands } from '../views/glyph/pathCommands'
import { registerSmoothCommands } from '../views/glyph/smoothCommands'

import { registerCommands, type Command } from './commands'
import {
  openBitmapTab,
  openFeaturesTab,
  openFontInfoTab,
  openGlyphTab,
} from './tabActions'

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
    // Through the preference store so the menu and the Preferences dialog
    // write the same record; it mirrors the choice into the shell itself.
    run: () => usePrefsStore.getState().update({ theme }),
  }
}

/**
 * A direct interface-language choice.
 *
 * Row 80 of the P5 checklist asks for an English-UI toggle: a reader who cannot
 * read the current language must be able to get back to English without finding
 * the right row of the Preferences dialog, so the languages are commands in the
 * View menu and the command palette as well, beside the theme choices.
 */
function languageCommand(language: Language): Command {
  return {
    id: `view.language.${language}`,
    title: `cmd.view.language.${language}`,
    menu: 'view',
    group: 'language',
    when: () => useUiStore.getState().language !== language,
    disabledReason: 'cmd.reason.languageActive',
    run: () => usePrefsStore.getState().update({ language }),
  }
}

async function runHistory(method: 'doc.undo' | 'doc.redo'): Promise<void> {
  const doc = activeDoc()
  if (doc === null) return
  // The label of the entry about to move: after the call it has swapped
  // stacks, and the status bar wants to name what just happened (§10).
  const summary = activeSummary()
  const moved =
    method === 'doc.undo' ? summary?.undo_label : summary?.redo_label
  await withSettledDocument(doc, () => call(method, { doc }))
  useUiStore.getState().flashUndoLabel(moved ?? null)
  await useSessionStore.getState().refreshSummary(doc)
}

/** Registers every built-in command. Safe to call more than once. */
export function registerBuiltinCommands(): void {
  registerCrossDocumentCommands()
  registerHelpCommands()
  registerAutomationCommands()
  registerFontViewCommands()
  registerGlyphViewCommands()
  registerPathCommands()
  registerSmoothCommands()
  registerLayerCommands()
  registerMetricsCommands()
  registerCommands([
    {
      id: 'bitmap.open',
      disabledReason: 'cmd.reason.noDocument',
      title: 'bitmap.title',
      menu: 'bitmap',
      group: 'editor',
      when: hasDocument,
      run: async () => {
        const doc = activeDoc()
        if (doc !== null) await openBitmapTab(doc)
      },
    },
    ...(['ArrowLeft', 'ArrowRight'] as const).map((key): Command => ({
      id: `text.kerning.${key}`,
      title: 'textControls.sample.kerning',
      shortcut: `Mod+Alt+${key}`,
      when: () => false,
      disabledReason: 'textControls.sample.kerning',
      run: () => undefined,
    })),
    {
      id: 'view.textPreview',
      title: 'textView.previewMenu',
      menu: 'view',
      group: 'panels',
      shortcut: 'Mod+3',
      run: () => useUiStore.getState().toggleTextPreview(),
    },
    {
      id: 'file.new',
      title: 'cmd.file.new',
      menu: 'file',
      group: 'open',
      shortcut: 'Mod+N',
      run: () => useUiStore.getState().setDialog({ kind: 'newFont' }),
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
      id: 'file.openProject',
      title: 'cmd.file.openProject',
      menu: 'file',
      group: 'open',
      run: () => useSessionStore.getState().openProjectDialog(),
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
      allowInInput: true,
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
      allowInInput: true,
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
      id: 'file.export',
      title: 'cmd.file.export',
      menu: 'file',
      group: 'export',
      shortcut: 'Mod+E',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDoc()
        if (doc !== null)
          useUiStore.getState().setDialog({ kind: 'export', doc })
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
      id: 'font.findProblems',
      title: 'cmd.font.findProblems',
      menu: 'font',
      group: 'validation',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDoc()
        if (doc !== null)
          useUiStore.getState().setDialog({ kind: 'findProblems', doc })
      },
    },
    {
      id: 'edit.undo',
      title: 'cmd.edit.undo',
      menu: 'edit',
      group: 'history',
      shortcut: 'Mod+Z',
      detail: () => activeSummary()?.undo_label ?? undefined,
      when: () => {
        const doc = activeDoc()
        return (
          activeSummary()?.undo_label != null ||
          (doc !== null && hasPendingDocumentDraft(doc))
        )
      },
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
      id: 'glyph.add',
      title: 'cmd.glyph.add',
      menu: 'glyph',
      group: 'glyph',
      shortcut: 'Mod+Shift+G',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDoc()
        if (doc !== null)
          useUiStore.getState().setDialog({ kind: 'addGlyphs', doc })
      },
    },
    {
      id: 'edit.history',
      title: 'cmd.view.history',
      menu: 'edit',
      group: 'history',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      // The timeline lives in the Inspector's section stack, so the command
      // opens the dock and that section rather than duplicating the list.
      run: () => {
        const ui = useUiStore.getState()
        ui.setPanelVisible('inspector', true)
        ui.setSection('history', true)
      },
    },
    {
      id: 'edit.preferences',
      title: 'cmd.edit.preferences',
      menu: 'edit',
      group: 'prefs',
      shortcut: 'Mod+,',
      run: () => useUiStore.getState().setDialog({ kind: 'preferences' }),
    },
    {
      id: 'view.features',
      title: 'cmd.view.features',
      menu: 'view',
      group: 'editor',
      when: hasDocument,
      disabledReason: 'cmd.reason.noDocument',
      run: async () => {
        const doc = activeDoc()
        if (doc !== null) await openFeaturesTab(doc)
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
    ...SUPPORTED_LANGUAGES.map(languageCommand),
    ...shellCommands(),
  ])
  // After the block above, so the Bitmap menu opens with the editor itself and
  // lists the strike tools and overlays under it.
  registerBitmapCommands()
}
