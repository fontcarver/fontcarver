/** Layer-local review marks drawn in logical pixels above the editable outline. */
import type { LayerAnnotation } from '../../api'
import type { Viewport } from '../../canvas/Viewport'
import type { GlyphTheme } from './glyphTheme'

/** Paint annotations without scaling their text or stroke widths with zoom. */
export function drawAnnotations(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  annotations: readonly LayerAnnotation[],
  theme: GlyphTheme,
): void {
  if (!annotations.length) return
  ctx.save()
  ctx.strokeStyle = theme.warning
  ctx.fillStyle = theme.warning
  ctx.lineWidth = 1.5
  ctx.font = `12px ${theme.fontFamily}`
  ctx.textBaseline = 'bottom'
  for (const annotation of annotations) {
    const start = viewport.toScreen(annotation)
    const end = viewport.toScreen({ x: annotation.end_x, y: annotation.end_y })
    ctx.beginPath()
    if (annotation.kind === 'Arrow') {
      const angle = Math.atan2(end.y - start.y, end.x - start.x)
      ctx.moveTo(start.x, start.y)
      ctx.lineTo(end.x, end.y)
      for (const direction of [-1, 1]) {
        ctx.moveTo(end.x, end.y)
        ctx.lineTo(
          end.x - 8 * Math.cos(angle + (direction * Math.PI) / 6),
          end.y - 8 * Math.sin(angle + (direction * Math.PI) / 6),
        )
      }
    } else if (annotation.kind === 'Circle') {
      ctx.ellipse(
        (start.x + end.x) / 2,
        (start.y + end.y) / 2,
        Math.abs(end.x - start.x) / 2,
        Math.abs(end.y - start.y) / 2,
        0,
        0,
        2 * Math.PI,
      )
    } else if (annotation.kind === 'Rectangle') {
      ctx.rect(
        Math.min(start.x, end.x),
        Math.min(start.y, end.y),
        Math.abs(end.x - start.x),
        Math.abs(end.y - start.y),
      )
    } else if (annotation.kind === 'Plus' || annotation.kind === 'Minus') {
      const radius = Math.max(6, Math.hypot(end.x - start.x, end.y - start.y))
      ctx.moveTo(start.x - radius, start.y)
      ctx.lineTo(start.x + radius, start.y)
      if (annotation.kind === 'Plus') {
        ctx.moveTo(start.x, start.y - radius)
        ctx.lineTo(start.x, start.y + radius)
      }
    } else {
      ctx.moveTo(start.x - 3, start.y)
      ctx.lineTo(start.x + 3, start.y)
      ctx.moveTo(start.x, start.y - 3)
      ctx.lineTo(start.x, start.y + 3)
    }
    ctx.stroke()
    const lines = annotation.text.split('\n')
    lines.forEach((line, index) => {
      ctx.fillText(
        line,
        start.x + 5,
        start.y - 5 - (lines.length - 1 - index) * 15,
      )
    })
  }
  ctx.restore()
}
