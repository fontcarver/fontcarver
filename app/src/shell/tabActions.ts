import { activeEditor } from '../editor/activeEditor'
import { settleDocumentEdits } from '../editor/documentPersistence'
import type { DocId, GlyphId } from '../api'
import { useSessionStore } from '../stores/sessionStore'
import { tabId, useUiStore } from '../stores/uiStore'
import { reportError } from './commands'

let transitionVersion = 0

/** Captures a navigation request before any asynchronous work can replace it. */
export function prepareTabTransition(): {
  ready: Promise<boolean>
  isCurrent: () => boolean
  settle: () => Promise<boolean>
} {
  const version = ++transitionVersion
  const initial = useUiStore.getState()
  const id = initial.activeTab
  const doc = initial.tabs.find((tab) => tab.id === id)?.doc
  const editor = activeEditor()
  const isCurrent = () => {
    const ui = useUiStore.getState()
    return (
      version === transitionVersion &&
      ui.activeTab === id &&
      ui.tabs.find((tab) => tab.id === id)?.doc === doc &&
      activeEditor() === editor
    )
  }
  const settle = async () => {
    try {
      if (!isCurrent()) return false
      if (doc !== undefined) await settleDocumentEdits(doc, isCurrent)
      return isCurrent()
    } catch (error) {
      reportError(error)
      return false
    }
  }
  return { ready: settle(), isCurrent, settle }
}

/** Selects a tab only after the old document's edits have settled. */
export async function selectTab(id: string): Promise<boolean> {
  const initial = useUiStore.getState()
  if (!initial.tabs.some((tab) => tab.id === id)) return false
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return false
  const ui = useUiStore.getState()
  if (!ui.tabs.some((tab) => tab.id === id)) return false
  ui.setActiveTab(id)
  return true
}

/** Cycles the shell tabs using the same edit handover as pointer selection. */
export async function cycleTab(delta: number): Promise<void> {
  const { tabs, activeTab } = useUiStore.getState()
  if (tabs.length === 0) return
  const index = tabs.findIndex((tab) => tab.id === activeTab)
  const next =
    (((Math.max(0, index) + delta) % tabs.length) + tabs.length) % tabs.length
  const target = tabs[next]
  if (target) await selectTab(target.id)
}

/**
 * Closes one tab. The Font tab represents the document itself, so closing it
 * closes the document (asking first when there are unsaved changes) and takes
 * every other tab of that document with it.
 */
export async function closeTab(id: string): Promise<void> {
  const ui = useUiStore.getState()
  const tab = ui.tabs.find((candidate) => candidate.id === id)
  if (!tab) return
  if (tab.kind === 'font') {
    await useSessionStore.getState().requestClose(tab.doc)
    return
  }
  if (ui.activeTab === id) {
    const transition = prepareTabTransition()
    if (!(await transition.ready) || !transition.isCurrent()) return
  }
  useUiStore.getState().closeTab(id)
}

function openSettledGlyphTab(doc: DocId, gid: GlyphId, label?: string): string {
  const ui = useUiStore.getState()
  const matching = ui.tabs.find(
    (tab) => tab.kind === 'glyph' && tab.doc === doc && tab.gid === gid,
  )
  if (matching) {
    ui.setActiveTab(matching.id)
    return matching.id
  }
  return ui.openTab({ kind: 'glyph', doc, gid, label })
}

/** Opens (or focuses) a Glyph tab after finishing the previously active edit. */
export async function openGlyphTab(
  doc: DocId,
  gid: GlyphId,
  label?: string,
): Promise<string | undefined> {
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return undefined
  return openSettledGlyphTab(doc, gid, label)
}

/**
 * Moves a glyph tab from one glyph to another. Stepping through glyphs must
 * not pile up tabs, so the new tab is opened and the old one closed.
 * The caller must finish the previous edit; UI navigation uses changeGlyphTab.
 */
export function replaceGlyphTab(
  doc: DocId,
  fromGid: GlyphId,
  toGid: GlyphId,
  label?: string,
): string {
  const ui = useUiStore.getState()
  const existing = ui.tabs.find((tab) => tab.id === ui.activeTab)
  const activeMatches =
    existing?.kind === 'glyph' &&
    existing.doc === doc &&
    existing.gid === fromGid
  if (activeMatches && existing.textState) {
    ui.updateTab(existing.id, {
      gid: toGid,
      label,
      textState: { ...existing.textState, activeIndex: null, mode: 'edit' },
    })
    ui.setActiveTab(existing.id)
    return existing.id
  }
  const next = openSettledGlyphTab(doc, toGid, label)
  const previous = activeMatches ? existing.id : tabId('glyph', doc, fromGid)
  if (next !== previous) ui.closeTab(previous)
  return next
}

/** Opens (or focuses) the Font Info tab of a document. */
export async function openFontInfoTab(doc: DocId): Promise<string | undefined> {
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return undefined
  return useUiStore.getState().openTab({ kind: 'fontinfo', doc })
}

/** Opens the document's Features editor after persisting the old view. */
export async function openFeaturesTab(doc: DocId): Promise<string | undefined> {
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return undefined
  return useUiStore.getState().openTab({ kind: 'features', doc })
}

/** Finishes old tools before a sidebar or keyboard step replaces its target. */
export async function changeGlyphTab(
  doc: DocId,
  from: GlyphId,
  to: GlyphId,
  label?: string,
  expectedTabId?: string,
): Promise<void> {
  const initial = useUiStore.getState()
  const id = initial.activeTab
  if (expectedTabId !== undefined && id !== expectedTabId) return
  const previous = initial.tabs.find((tab) => tab.id === id)
  if (previous?.doc !== doc || previous.gid !== from) return
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return
  const ui = useUiStore.getState()
  const tab = ui.tabs.find((tab) => tab.id === id)
  if (ui.activeTab !== id || tab?.doc !== doc || tab.gid !== from) return
  replaceGlyphTab(doc, from, to, label)
}
