import type { Viewport } from '../../canvas/Viewport'
import type { GlyphTheme } from './glyphTheme'

/** Parsed preview data, reused while panning or drawing the source. */
export interface VariationCanvasPath {
  path: Path2D
  width: number
}

/** Draw an interpolated outline and advance alongside editable source nodes. */
export function drawVariationPreview(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  preview: VariationCanvasPath,
  theme: GlyphTheme,
): void {
  ctx.save()
  ctx.translate(viewport.offset.x, viewport.offset.y)
  ctx.scale(viewport.scale, -viewport.scale)
  ctx.fillStyle = theme.accent
  ctx.strokeStyle = theme.accent
  ctx.globalAlpha = 0.12
  ctx.fill(preview.path, 'nonzero')
  ctx.globalAlpha = 0.85
  ctx.lineWidth = 1.5 / viewport.scale
  ctx.stroke(preview.path)
  const tick = 12 / viewport.scale
  ctx.beginPath()
  ctx.moveTo(preview.width, -tick)
  ctx.lineTo(preview.width, tick)
  ctx.stroke()
  ctx.restore()
}
