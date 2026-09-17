import type { Viewport } from '../../canvas/Viewport'
import type { GlyphTheme } from '../glyph/glyphTheme'
import type { TextCanvasRun } from './textCanvas'

/** Draws gap handles in screen space at unchanged kernel positions. */
export function drawTextGaps(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  run: TextCanvasRun,
  theme: GlyphTheme,
  visible: readonly number[],
): void {
  if (run.mode !== 'text' || !run.gaps) return
  const shown = new Set(visible)
  ctx.save()
  ctx.setTransform(viewport.dpr, 0, 0, viewport.dpr, 0, 0)
  for (const gap of run.gaps) {
    if (
      gap.reason !== null ||
      (!shown.has(gap.index) && !shown.has(gap.index + 1))
    )
      continue
    const point = viewport.toScreen({
      x: gap.x - run.activeOrigin.x,
      y: -run.unitsPerEm * 0.12 - run.activeOrigin.y,
    })
    const selected = run.selectedGap === gap.index
    ctx.fillStyle = selected ? theme.accent : theme.metricStrong
    if (selected) {
      const top = viewport.toScreen({
        x: gap.x - run.activeOrigin.x,
        y: run.unitsPerEm * 0.8 - run.activeOrigin.y,
      })
      ctx.strokeStyle = theme.accent
      ctx.lineWidth = 1
      ctx.setLineDash([3, 3])
      ctx.beginPath()
      ctx.moveTo(point.x, point.y)
      ctx.lineTo(top.x, top.y)
      ctx.stroke()
    }
    const radius = selected ? 5 : 3
    ctx.beginPath()
    ctx.moveTo(point.x, point.y - radius)
    ctx.lineTo(point.x + radius, point.y)
    ctx.lineTo(point.x, point.y + radius)
    ctx.lineTo(point.x - radius, point.y)
    ctx.closePath()
    ctx.fill()
  }
  ctx.restore()
}
