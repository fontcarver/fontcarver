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
  for (const glyph of glyphs) {
    context.save()
    context.translate(glyph.x, glyph.y)
    context.fill(glyph.path)
    context.restore()
  }
  context.restore()
}
