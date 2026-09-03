/**
 * Glyph-name registry.
 *
 * The grid selects by `GlyphId`, but commands such as "Copy Glyph Names" need
 * the names. Every cell the grid loads — from a page request, a range fetch or
 * a select-all — records its name here, so the names of a selection are
 * available without a further round trip. Names only change when the glyph
 * order changes, which clears the document's entry.
 */

import type { DocId, GlyphCell, GlyphId } from '../../api'
import { onDocumentForgotten } from '../../stores/documentStore'

const byDoc = new Map<DocId, Map<GlyphId, string>>()

/** Records the names of freshly loaded cells. */
export function rememberGlyphNames(
  doc: DocId,
  cells: readonly GlyphCell[],
): void {
  if (cells.length === 0) return
  let names = byDoc.get(doc)
  if (!names) {
    names = new Map()
    byDoc.set(doc, names)
  }
  for (const cell of cells) names.set(cell.gid, cell.name)
}

/** The known name of one glyph, or `undefined` when it was never loaded. */
export function glyphName(doc: DocId, gid: GlyphId): string | undefined {
  return byDoc.get(doc)?.get(gid)
}

/** The known names of a selection, in the given order, skipping unknown ones. */
export function glyphNamesOf(doc: DocId, gids: Iterable<GlyphId>): string[] {
  const names = byDoc.get(doc)
  if (!names) return []
  const found: string[] = []
  for (const gid of gids) {
    const name = names.get(gid)
    if (name !== undefined) found.push(name)
  }
  return found
}

/** Forgets one document's names; called when it closes or is reordered. */
export function forgetGlyphNames(doc: DocId): void {
  byDoc.delete(doc)
}

onDocumentForgotten(forgetGlyphNames)
