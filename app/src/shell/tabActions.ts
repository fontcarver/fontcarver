import type { DocId, GlyphId } from '../api'
import { useSessionStore } from '../stores/sessionStore'
import { tabId, useUiStore } from '../stores/uiStore'

/**
 * Closes one tab. The Font tab represents the document itself, so closing it
 * closes the document (asking first when there are unsaved changes) and takes
 * every other tab of that document with it.
 */
export function closeTab(id: string): void {
  const ui = useUiStore.getState()
  const tab = ui.tabs.find((candidate) => candidate.id === id)
  if (!tab) return
  if (tab.kind === 'font') {
    void useSessionStore.getState().requestClose(tab.doc)
    return
  }
  ui.closeTab(id)
}

/** Opens (or focuses) the Glyph tab for one glyph of a document. */
export function openGlyphTab(doc: DocId, gid: GlyphId, label?: string): string {
  return useUiStore.getState().openTab({ kind: 'glyph', doc, gid, label })
}

/**
 * Moves a glyph tab from one glyph to another. Stepping through glyphs must
 * not pile up tabs, so the new tab is opened and the old one closed.
 */
export function replaceGlyphTab(
  doc: DocId,
  fromGid: GlyphId,
  toGid: GlyphId,
  label?: string,
): string {
  const ui = useUiStore.getState()
  const previous = tabId('glyph', doc, fromGid)
  const existing = ui.tabs.find(
    (tab) => tab.kind === 'glyph' && tab.doc === doc && tab.gid === fromGid,
  )
  if (existing?.textState) {
    ui.updateTab(existing.id, { gid: toGid, label })
    ui.setActiveTab(existing.id)
    return existing.id
  }
  const next = openGlyphTab(doc, toGid, label)
  if (next !== previous) ui.closeTab(previous)
  return next
}

/** Opens (or focuses) the Font Info tab of a document. */
export function openFontInfoTab(doc: DocId): string {
  return useUiStore.getState().openTab({ kind: 'fontinfo', doc })
}
