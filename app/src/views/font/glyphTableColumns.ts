/** Available metadata columns in the font overview table. */
export type GlyphTableColumn =
  'name' | 'unicode' | 'width' | 'category' | 'script' | 'status'

/** Initial display order for a document's glyph table. */
export const DEFAULT_GLYPH_TABLE_COLUMNS: readonly GlyphTableColumn[] = [
  'name',
  'unicode',
  'width',
  'category',
  'script',
  'status',
]

const COLUMN_WIDTHS: Record<GlyphTableColumn, string> = {
  name: 'minmax(180px, 2fr)',
  unicode: 'minmax(140px, 1.3fr)',
  width: '100px',
  category: '120px',
  script: '90px',
  status: 'minmax(140px, 1.5fr)',
}

/** Shared grid tracks keep table headers and virtualized rows aligned. */
export function glyphTableTemplate(
  columns: readonly GlyphTableColumn[],
): string {
  return columns.map((column) => COLUMN_WIDTHS[column]).join(' ')
}
