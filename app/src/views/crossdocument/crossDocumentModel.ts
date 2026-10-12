import { call, type DocId, type GlyphCell, type GlyphId } from '../../api'
import { emptyFilter } from '../../stores/documentStore'

/** Parse an explicit ordered set without silently dropping duplicate names. */
export function parseGlyphNames(text: string): string[] {
  const names = text
    .trim()
    .split(/[\s,]+/)
    .filter(Boolean)
  if (new Set(names).size !== names.length) throw new Error('duplicate')
  return names
}

/** Preserve the user's source and target ordering for selected-glyph pastes. */
export function mapPasteTargets(
  source: readonly GlyphId[],
  target: readonly GlyphId[],
  destination: 'same_name' | 'selected' | 'components',
) {
  if (destination !== 'same_name' && source.length !== target.length)
    throw new Error('count')
  return source.map((gid, index) => ({
    source: gid,
    target: destination === 'same_name' ? null : (target[index] ?? null),
  }))
}

/** Read inventory pages without requesting expensive outline geometry. */
export async function documentGlyphs(doc: DocId): Promise<GlyphCell[]> {
  const glyphs: GlyphCell[] = []
  for (;;) {
    const page = await call('glyphs.list', {
      doc,
      master: null,
      filter: emptyFilter(),
      sort: 'Order',
      offset: glyphs.length,
      limit: 2000,
      include_paths: false,
    })
    glyphs.push(...page.items)
    if (glyphs.length >= page.total || page.items.length === 0) return glyphs
  }
}
