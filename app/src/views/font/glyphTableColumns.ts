import type { GlyphSort } from '../../api'

/** Available metadata columns in the font overview table. */
export type GlyphTableColumn =
  | 'name'
  | 'unicode'
  | 'width'
  | 'category'
  | 'subCategory'
  | 'script'
  | 'status'

/** Every column the table can show, in canonical order. */
export const GLYPH_TABLE_COLUMNS: readonly GlyphTableColumn[] = [
  'name',
  'unicode',
  'width',
  'category',
  'subCategory',
  'script',
  'status',
]

/** Initial display order for a document's glyph table. */
export const DEFAULT_GLYPH_TABLE_COLUMNS: readonly GlyphTableColumn[] = [
  'name',
  'unicode',
  'width',
  'category',
  'script',
  'status',
]

/**
 * The kernel order each column sorts by.
 *
 * Every column has one: `Category` orders by category then subcategory, so the
 * subcategory column shares it, and `Status` orders by badge weight so the
 * glyphs needing attention come first.
 */
export const COLUMN_SORTS: Record<GlyphTableColumn, GlyphSort> = {
  name: 'Name',
  unicode: 'Unicode',
  width: 'Width',
  category: 'Category',
  subCategory: 'Category',
  script: 'Script',
  status: 'Status',
}

const COLUMN_WIDTHS: Record<GlyphTableColumn, string> = {
  name: 'minmax(180px, 2fr)',
  unicode: 'minmax(140px, 1.3fr)',
  width: '100px',
  category: '120px',
  subCategory: '120px',
  script: '90px',
  status: 'minmax(140px, 1.5fr)',
}

/** Shared grid tracks keep table headers and virtualized rows aligned. */
export function glyphTableTemplate(
  columns: readonly GlyphTableColumn[],
): string {
  return columns.map((column) => COLUMN_WIDTHS[column]).join(' ')
}
