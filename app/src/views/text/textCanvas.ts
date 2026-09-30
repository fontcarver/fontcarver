import type { GlyphCell, GlyphId } from '../../api'
import type { FontBox, Point2 } from '../../canvas/Viewport'
import type { ColorGlyphPaint } from '../../canvas/colorGlyph'

import type { PositionedGlyph } from './textLayout'
import type { TextGap } from './textGaps'

/** Shaped-run state consumed by the existing glyph canvas. */
export interface TextCanvasRun {
  glyphs: readonly PositionedGlyph[]
  cells: ReadonlyMap<GlyphId, GlyphCell>
  colors?: ReadonlyMap<GlyphId, ColorGlyphPaint>
  activeIndex: number | null
  activeOrigin: Point2
  mode: 'text' | 'edit'
  unitsPerEm: number
  fitBox: FontBox
  requestedScale: number
  gaps?: readonly TextGap[]
  selectedGap?: number | null
  /** Resolved stored value for the selected boundary, never a draft value. */
  selectedKerningValue?: number | null
  /** Bounded group-member outlines at the selected pair's glyph origins. */
  kerningGhosts?: readonly TextKerningGhost[]
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

/** One passive outline or color glyph drawn around the editable glyph. */
export interface SurroundingGlyph {
  index: number
  path: Path2D | null
  color?: ColorGlyphPaint
  x: number
  y: number
  ghost?: boolean
}

/** Unparsed group-member outline, positioned in absolute shaped-run space. */
export interface TextKerningGhost {
  index: number
  name: string
  path: string
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
