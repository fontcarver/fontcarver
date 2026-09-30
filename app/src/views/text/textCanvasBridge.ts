import type { GlyphId } from '../../api'
import type { FontBox, Point2, Viewport } from '../../canvas/Viewport'

import type { SurroundingGlyph, TextCanvasRun } from './textCanvas'
import { hitTextGap } from './textGaps'
import {
  hitTextGlyph,
  visibleGlyphIndexes,
  type TextGlyphBounds,
} from './textRunGeometry'

/** Bboxes available for the run without loading any offscreen outline. */
export function textRunBounds(
  run: Pick<TextCanvasRun, 'cells'>,
): TextGlyphBounds {
  return new Map(
    [...run.cells].flatMap(([gid, cell]) =>
      cell.bbox
        ? ([[gid, cell.bbox]] as [GlyphId, [number, number, number, number]][])
        : [],
    ),
  )
}

/** Parses and positions the cells already selected by the bounded loader. */
export function surroundingGlyphs(
  run: TextCanvasRun,
  paths: Map<string, Path2D>,
  indexes: readonly number[],
): SurroundingGlyph[] {
  const ghosts: SurroundingGlyph[] =
    run.mode === 'text'
      ? (run.kerningGhosts ?? []).flatMap((ghost) => {
          if (!indexes.includes(ghost.index)) return []
          let path = paths.get(ghost.path)
          if (!path) {
            try {
              path = new Path2D(ghost.path)
            } catch {
              return []
            }
            paths.set(ghost.path, path)
          }
          return [
            {
              index: ghost.index,
              path,
              x: ghost.x - run.activeOrigin.x,
              y: ghost.y - run.activeOrigin.y,
              ghost: true,
            },
          ]
        })
      : []
  return [
    ...ghosts,
    ...indexes.flatMap((index) => {
      const glyph = run.glyphs[index]
      if (!glyph) return []
      if (run.mode === 'edit' && index === run.activeIndex) return []
      if (glyph.documentGid === null) return []
      const data = run.cells.get(glyph.documentGid)?.path
      const color = run.colors?.get(glyph.documentGid)
      if (!data && !color) return []
      let path = data ? (paths.get(data) ?? null) : null
      if (data && !path) {
        try {
          path = new Path2D(data)
        } catch {
          if (!color) return []
        }
        if (path) paths.set(data, path)
      }
      return [
        {
          index,
          path,
          ...(color ? { color } : {}),
          x: glyph.x - run.activeOrigin.x,
          y: glyph.y - run.activeOrigin.y,
        },
      ]
    }),
  ]
}

/** Finds a run glyph under a canvas point in absolute shaped-run space. */
export function hitCanvasRun(
  run: TextCanvasRun,
  viewport: Viewport,
  screen: Point2,
): number | null {
  const local = viewport.toFont(screen)
  return hitTextGlyph(
    run.glyphs,
    textRunBounds(run),
    { x: local.x + run.activeOrigin.x, y: local.y + run.activeOrigin.y },
    run.unitsPerEm,
    viewport.scale,
  )
}

/** Finds an editable gap after applying the active glyph's local origin. */
export function hitCanvasGap(
  run: TextCanvasRun,
  viewport: Viewport,
  screen: Point2,
): number | null {
  if (run.mode !== 'text') return null
  const local = viewport.toFont(screen)
  return hitTextGap(
    run.gaps ?? [],
    { x: local.x + run.activeOrigin.x, y: local.y + run.activeOrigin.y },
    run.unitsPerEm,
    viewport.scale,
  )
}

/** Returns the bounded set that should have cell paths for this viewport. */
export function visibleCanvasRun(
  run: TextCanvasRun,
  viewport: Viewport,
): number[] {
  return visibleGlyphIndexes(
    run.glyphs,
    textRunBounds(run),
    viewport,
    run.activeOrigin,
    run.unitsPerEm,
  )
}

/** Converts absolute shaped-run bounds into the active glyph's local space. */
export function localRunBox(run: TextCanvasRun): FontBox {
  return {
    x0: run.fitBox.x0 - run.activeOrigin.x,
    y0: run.fitBox.y0 - run.activeOrigin.y,
    x1: run.fitBox.x1 - run.activeOrigin.x,
    y1: run.fitBox.y1 - run.activeOrigin.y,
  }
}

/** Keeps the absolute run fixed when its glyph-local editing origin changes. */
export function shiftRunOrigin(
  viewport: Viewport,
  previous: Point2,
  next: Point2,
): void {
  viewport.pan(
    (next.x - previous.x) * viewport.scale,
    (previous.y - next.y) * viewport.scale,
  )
}
