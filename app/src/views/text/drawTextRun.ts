import type { Viewport } from '../../canvas/Viewport'

import type { SurroundingGlyph } from './textCanvas'

/** Draws passive shaped glyphs in the active glyph's local viewport. */
export function drawTextRun(
  context: CanvasRenderingContext2D,
  viewport: Viewport,
  glyphs: readonly SurroundingGlyph[],
  ink: string,
): void {
  if (glyphs.length === 0) return
  context.save()
  context.setTransform(
    viewport.scale * viewport.dpr,
    0,
    0,
    -viewport.scale * viewport.dpr,
    viewport.offset.x * viewport.dpr,
    viewport.offset.y * viewport.dpr,
  )
  context.fillStyle = ink
  context.strokeStyle = ink
  for (const glyph of glyphs) {
    context.save()
    context.translate(glyph.x, glyph.y)
    if (glyph.ghost && glyph.path) {
      context.globalAlpha = 0.35
      context.lineWidth = 1 / viewport.scale
      context.setLineDash([3 / viewport.scale, 3 / viewport.scale])
      context.stroke(glyph.path)
    } else if (glyph.color) glyph.color.paint(context, ink)
    else if (glyph.path) context.fill(glyph.path)
    context.restore()
  }
  context.restore()
}
