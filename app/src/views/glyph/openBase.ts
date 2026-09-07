/**
 * Opening the base glyph of a component.
 *
 * Components address their base by *name* while tabs address glyphs by session
 * handle, so the name is resolved through `glyphs.search` — the same lookup the
 * export report's glyph chips use — and the exact match is opened in its own
 * Edit tab. A name the search cannot place (a component whose base was renamed
 * out from under it) opens nothing rather than guessing.
 */

import { call, type DocId } from '../../api'
import { openGlyphTab } from '../../shell/tabActions'

/** Opens the Edit tab of one glyph by name; resolves to `false` when unknown. */
export async function openGlyphByName(
  doc: DocId,
  name: string,
): Promise<boolean> {
  const cells = await call('glyphs.search', { doc, query: name, limit: 8 })
  const cell = cells.find((candidate) => candidate.name === name)
  if (!cell) return false
  openGlyphTab(doc, cell.gid, cell.name)
  return true
}
