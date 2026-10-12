import type { GlyphFilter } from '../../api'
import { emptyFilter, isFiltered } from '../../stores/documentStore'

/** Imports whitespace-separated glyph names, preserving first-occurrence order. */
export function parseGlyphList(text: string): string[] {
  return [...new Set(text.split(/\s+/u).filter(Boolean))]
}

/** Matches exactly the names in a list, including an empty list. */
export function glyphListFilter(glyphs: string[]): GlyphFilter {
  return { ...emptyFilter(), glyph_names: glyphs }
}

/** A list is active only while no extra condition narrows its membership. */
export function isGlyphListActive(
  filter: GlyphFilter,
  glyphs: readonly string[],
): boolean {
  const selected = filter.glyph_names
  if (selected == null || selected.length !== glyphs.length) return false
  const names = new Set(selected)
  return (
    glyphs.every((name) => names.has(name)) &&
    !isFiltered({ ...filter, glyph_names: null })
  )
}
