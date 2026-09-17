import type { GlyphCell, GlyphId } from '../../api'
import type { FontBox, Point2 } from '../../canvas/Viewport'

import type { PositionedGlyph } from './textLayout'
import type { TextGap } from './textGaps'

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
  gaps?: readonly TextGap[]
  selectedGap?: number | null
  onSelectGap?: (index: number) => void
  onKerningKey?: (
    event: KeyboardEvent | React.KeyboardEvent<HTMLCanvasElement>,
  ) => boolean
  onMetricKey?: (
    event: KeyboardEvent | React.KeyboardEvent<HTMLCanvasElement>,
  ) => boolean
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

declare global {
  var __fwTextRun:
    | {
        glyphs: readonly PositionedGlyph[]
        activeIndex: number | null
        selectedGap: number | null
        activeOrigin: { x: number; y: number }
      }
    | null
    | undefined
}
