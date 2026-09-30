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
  /** Whether the selected boundary's winning rule departs from its class rule. */
  selectedKerningException?: boolean
  /** Boundary indexes whose winning rule is an exception to a class rule. */
  kerningExceptions?: ReadonlySet<number>
  /** Measurement beam drawn across the run, with the margins measured on it. */
  beam?: TextSpacingBeam | null
  /** Band, in font units, outside which the run is hidden. */
  cutOff?: TextCutOffBand | null
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
  /**
   * The pointer grabbed the measurement beam on the canvas.
   *
   * The three beam hooks are the canvas half of dragging the beam: the glyph
   * canvas owns the geometry — it knows where the line is on screen and what
   * height a pointer position means — and the run owns what that height does.
   * `onBeamGrab` is only called while `beam` is present, and a grab is always
   * followed by `onBeamRelease`, drag or no drag.
   */
  onBeamGrab?: () => void
  /** The beam moved, in font units y-up from the baseline. */
  onBeamDrag?: (height: number) => void
  /** The beam drag ended. */
  onBeamRelease?: () => void
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

/** A spacing beam: one measurement height and the margins measured on it. */
export interface TextSpacingBeam {
  /** Measurement height in font units, y-up from the baseline. */
  height: number
  /** Left and right margins at `height`, keyed by document glyph name. */
  margins: ReadonlyMap<string, { left: number; right: number }>
}

/** A cut-off display band in font units; only this slice of the run is shown. */
export interface TextCutOffBand {
  from: number
  to: number
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
