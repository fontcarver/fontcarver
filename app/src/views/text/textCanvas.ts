import type { GlyphCell, GlyphId } from '../../api'
import type { FontBox, Point2 } from '../../canvas/Viewport'

import type { PositionedGlyph } from './textLayout'

/** Shaped-run state consumed by the existing glyph canvas. */
export interface TextCanvasRun {
  glyphs: readonly PositionedGlyph[]
  cells: ReadonlyMap<GlyphId, GlyphCell>
  activeIndex: number | null
  activeOrigin: Point2
  mode: 'text' | 'edit'
  unitsPerEm: number
  fitBox: FontBox
  requestedScale: number
  onActivate: (index: number) => void
  onExitEdit: () => void
  onVisibleIndexes: (indexes: readonly number[]) => void
}

/** One parsed passive outline drawn around the editable glyph. */
export interface SurroundingGlyph {
  index: number
  path: Path2D
  x: number
  y: number
}
