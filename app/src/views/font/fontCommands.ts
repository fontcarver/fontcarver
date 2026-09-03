/**
 * Commands owned by the font view.
 *
 * They live here rather than in the shell so the view keeps its own vocabulary,
 * but they are registered in the same registry, so they appear in the menu bar,
 * the command palette and the keyboard bindings like every other command.
 */

import { type DocId, type GlyphId } from '../../api'
import { i18n } from '../../i18n'
import { registerCommands, type Command } from '../../shell/commands'
import {
  emptyFilter,
  useDocumentStore,
  type DocView,
} from '../../stores/documentStore'
import { useSessionStore } from '../../stores/sessionStore'
import { selectActiveTab, useUiStore } from '../../stores/uiStore'

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

/** Registers every command of the font view. Safe to call more than once. */
export function registerFontViewCommands(): void {
  const commands: Command[] = [
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
