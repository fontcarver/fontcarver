import { call, type DocId, type GlyphFilter, type MasterId } from '../../api'

const ALL_ENCODED_FILTER: GlyphFilter = {
  query: null,
  categories: [],
  scripts: [],
  blocks: [],
  unencoded_only: false,
  color_labels: [],
  tags: [],
  unicode_ranges: [],
  has_components: null,
  changed_only: false,
}

/** Reads encoded characters without paths, abandoning stale sample requests. */
export async function allEncodedText(
  doc: DocId,
  master: MasterId | null,
  isCurrent: () => boolean,
): Promise<string | null> {
  if (!isCurrent()) return null
  const ids = await call('glyphs.ids', {
    doc,
    master,
    filter: ALL_ENCODED_FILTER,
    sort: 'Unicode',
  })
  const codepoints = new Set<number>()
  for (let offset = 0; offset < ids.length; offset += 2000) {
    if (!isCurrent()) return null
    const page = await call('glyphs.list', {
      doc,
      master,
      filter: ALL_ENCODED_FILTER,
      sort: 'Unicode',
      offset,
      limit: 2000,
      include_paths: false,
    })
    if (!isCurrent()) return null
    for (const cell of page.items)
      for (const cp of cell.unicodes) codepoints.add(cp)
  }
  return isCurrent()
    ? [...codepoints]
        .map((cp) => (cp === 47 ? '//' : String.fromCodePoint(cp)))
        .join('')
    : null
}
