/**
 * Commands owned by the font view.
 *
 * They live here rather than in the shell so the view keeps its own vocabulary,
 * but they are registered in the same registry, so they appear in the menu bar,
 * the command palette and the keyboard bindings like every other command.
 */

import {
  call,
  type Command as KernelCommand,
  type DocId,
  type GlyphId,
} from '../../api'
import { setGlyphMetaCommand } from '../glyph/panels/panelCommands'
import { i18n } from '../../i18n'
import { registerCommands, type Command } from '../../shell/commands'
import {
  emptyFilter,
  useDocumentStore,
  type DocView,
} from '../../stores/documentStore'
import { useSessionStore } from '../../stores/sessionStore'
import { reportError, selectActiveTab, useUiStore } from '../../stores/uiStore'

import { COLOR_LABELS, colorLabelKey } from './colorLabels'
import { glyphNamesOf } from './glyphNames'
import { fetchGidRange, type GlyphQuery } from './glyphSelection'

/** The document of the active tab, but only while that tab is a Font tab. */
function activeFontDoc(): DocId | null {
  const tab = selectActiveTab(useUiStore.getState())
  return tab?.kind === 'font' ? tab.doc : null
}

function viewOf(doc: DocId): DocView | undefined {
  return useDocumentStore.getState().byDoc[doc]
}

function selectionOf(doc: DocId): ReadonlySet<GlyphId> {
  return viewOf(doc)?.selection ?? new Set<GlyphId>()
}

/** Rebuilds the query the grid is currently showing. */
function queryFor(doc: DocId): GlyphQuery {
  const view = viewOf(doc)
  const summary = useSessionStore
    .getState()
    .docs.find((entry) => entry.doc === doc)
  return {
    doc,
    master: view?.master ?? summary?.masters[0]?.id ?? null,
    filter: view?.filter ?? emptyFilter(),
    sort: view?.sort ?? 'Order',
  }
}

/** Copies text, falling back to a hidden textarea where the API is blocked. */
export async function writeClipboard(text: string): Promise<boolean> {
  try {
    if (globalThis.navigator?.clipboard) {
      await globalThis.navigator.clipboard.writeText(text)
      return true
    }
  } catch {
    // Fall through to the legacy path below.
  }
  const target = globalThis.document
  if (!target?.body) return false
  const area = target.createElement('textarea')
  area.value = text
  area.setAttribute('readonly', '')
  area.style.position = 'fixed'
  area.style.opacity = '0'
  target.body.appendChild(area)
  area.select()
  let copied: boolean
  try {
    copied = target.execCommand('copy')
  } catch {
    copied = false
  }
  target.body.removeChild(area)
  return copied
}

/**
 * Applies one colour label, or clears it, across the whole grid selection.
 *
 * One `doc.apply` with a batch of `SetGlyphMeta` commands, so the whole gesture
 * is a single undo step however many glyphs are selected. Names come from the
 * grid's cache, which already holds every selected cell.
 */
async function setSelectionColorLabel(
  doc: DocId,
  colorLabel: number | null,
): Promise<void> {
  const names = glyphNamesOf(doc, selectionOf(doc))
  if (names.length === 0) return
  const commands: KernelCommand[] = names.map((glyph) =>
    setGlyphMetaCommand(glyph, { colorLabel }),
  )
  await call('doc.apply', {
    doc,
    command: commands.length === 1 ? commands[0]! : { Batch: commands },
    label: i18n.t('editor.undo.setColorLabel'),
  })
}

/** Re-applies the glyph database to the selection in one undo step. */
async function updateSelectionGlyphInfo(doc: DocId): Promise<void> {
  const glyphs = [...selectionOf(doc)]
  if (glyphs.length === 0) return
  // The kernel checks the revision, so read it from the same source the glyph
  // database panel does rather than trusting the last event the view saw.
  const state = await call('glyphdata.state', { doc, gid: glyphs[0]! })
  await call('glyphdata.update', {
    doc,
    glyphs,
    expected_generation: state.generation,
    label: i18n.t('glyphDatabase.update'),
  })
  useUiStore.getState().pushToast({
    kind: 'info',
    message: i18n.t('toast.glyphInfoUpdated', { count: glyphs.length }),
  })
}

/** Registers every command of the font view. Safe to call more than once. */
export function registerFontViewCommands(): void {
  const hasSelection = () => {
    const doc = activeFontDoc()
    return doc !== null && selectionOf(doc).size > 0
  }
  const commands: Command[] = [
    ...COLOR_LABELS.map((label): Command => ({
      id: `glyph.colorLabel.${colorLabelKey(label.index)}`,
      title: `fontView.color.${colorLabelKey(label.index)}`,
      menu: 'glyph',
      group: 'colorLabel',
      when: hasSelection,
      disabledReason: 'cmd.reason.noSelection',
      run: () => {
        const doc = activeFontDoc()
        if (doc === null) return
        return setSelectionColorLabel(doc, label.index).catch(reportError)
      },
    })),
    {
      id: 'glyph.colorLabel.none',
      title: 'panel.glyph.noColor',
      menu: 'glyph',
      group: 'colorLabel',
      when: hasSelection,
      disabledReason: 'cmd.reason.noSelection',
      run: () => {
        const doc = activeFontDoc()
        if (doc === null) return
        return setSelectionColorLabel(doc, null).catch(reportError)
      },
    },
    {
      id: 'glyph.updateInfo',
      title: 'cmd.glyph.updateInfo',
      menu: 'glyph',
      group: 'glyph',
      when: hasSelection,
      disabledReason: 'cmd.reason.noSelection',
      run: () => {
        const doc = activeFontDoc()
        if (doc === null) return
        return updateSelectionGlyphInfo(doc).catch(reportError)
      },
    },
    {
      id: 'font.styleEffects',
      title: 'styleEffects.title',
      menu: 'font',
      group: 'styles',
      when: () => {
        const doc = activeFontDoc()
        return (
          doc !== null &&
          selectionOf(doc).size > 0 &&
          selectionOf(doc).size <= 512
        )
      },
      disabledReason: 'styleEffects.selectFirst',
      run: () => {
        const doc = activeFontDoc()
        if (doc === null) return
        const master = queryFor(doc).master
        if (!master) return
        useUiStore.getState().setDialog({
          kind: 'styleEffects',
          doc,
          glyphs: [...selectionOf(doc)],
          master,
        })
      },
    },
    {
      id: 'edit.selectAll',
      title: 'cmd.edit.selectAll',
      menu: 'edit',
      group: 'selection',
      shortcut: 'Mod+A',
      when: () => activeFontDoc() !== null,
      disabledReason: 'cmd.reason.noFontTab',
      run: async () => {
        const doc = activeFontDoc()
        if (doc === null) return
        const total = viewOf(doc)?.matchCount ?? 0
        if (total === 0) return
        const gids = await fetchGidRange(queryFor(doc), 0, total)
        useDocumentStore.getState().setSelection(doc, gids)
      },
    },
    {
      id: 'edit.selectNone',
      title: 'cmd.edit.selectNone',
      menu: 'edit',
      group: 'selection',
      when: () => {
        const doc = activeFontDoc()
        return doc !== null && selectionOf(doc).size > 0
      },
      disabledReason: 'cmd.reason.noSelection',
      run: () => {
        const doc = activeFontDoc()
        if (doc !== null) useDocumentStore.getState().clearSelection(doc)
      },
    },
    {
      id: 'edit.copyGlyphNames',
      title: 'cmd.edit.copyGlyphNames',
      menu: 'edit',
      group: 'selection',
      shortcut: 'Mod+Shift+C',
      when: () => {
        const doc = activeFontDoc()
        return doc !== null && selectionOf(doc).size > 0
      },
      disabledReason: 'cmd.reason.noSelection',
      run: async () => {
        const doc = activeFontDoc()
        if (doc === null) return
        const names = glyphNamesOf(doc, selectionOf(doc))
        if (names.length === 0) return
        const copied = await writeClipboard(names.join(' '))
        useUiStore.getState().pushToast({
          kind: copied ? 'info' : 'error',
          message: copied
            ? i18n.t('toast.copiedNames', { count: names.length })
            : i18n.t('toast.copyFailed'),
        })
      },
    },
  ]
  registerCommands(commands)
}
