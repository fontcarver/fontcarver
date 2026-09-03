/**
 * Anchor drawing: the named attachment points a layer carries, shown as a
 * small ring with a crosshair and the anchor's name.
 */

import type { Anchor } from '../../api'
import type { Viewport } from '../../canvas/Viewport'

import type { GlyphTheme } from './glyphTheme'

const RADIUS = 4
const ARM = 7
const LABEL_FONT_SIZE = 10

/** What `drawAnchors` renders. */
export interface AnchorsData {
  anchors: readonly Anchor[]
  showLabels: boolean
}

/** Draws the anchors of one layer. */
export function drawAnchors(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: AnchorsData,
  theme: GlyphTheme,
): void {
  if (data.anchors.length === 0) return
  ctx.save()
  ctx.strokeStyle = theme.anchor
  ctx.fillStyle = theme.anchor
  ctx.lineWidth = 1
  ctx.font = `${LABEL_FONT_SIZE}px ${theme.fontFamily}`
  ctx.textAlign = 'center'
  ctx.textBaseline = 'bottom'

  for (const anchor of data.anchors) {
    const at = viewport.toScreen({ x: anchor.x, y: anchor.y })
    ctx.beginPath()
    ctx.moveTo(at.x - ARM, at.y)
    ctx.lineTo(at.x + ARM, at.y)
    ctx.moveTo(at.x, at.y - ARM)
    ctx.lineTo(at.x, at.y + ARM)
    ctx.stroke()
    ctx.beginPath()
    ctx.arc(at.x, at.y, RADIUS, 0, Math.PI * 2)
    ctx.stroke()
    if (data.showLabels && anchor.name)
      ctx.fillText(anchor.name, at.x, at.y - ARM - 2)
  }
  ctx.restore()
}
