/**
 * Guide drawing.
 *
 * A guide is a point and an angle in font units, so it is an infinite line;
 * it is clipped to the canvas by extending it far enough in both directions.
 * Master guides are font-wide and drawn in the global colour, layer guides in
 * the local one, following the convention every incumbent editor uses.
 */

import type { Viewport } from '../../canvas/Viewport'

import type { SceneGuide } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'

const LABEL_FONT_SIZE = 10

/** What `drawGuides` renders. */
export interface GuidesData {
  guides: readonly SceneGuide[]
  showLabels: boolean
}

/** Draws the master and layer guides of a scene. */
export function drawGuides(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: GuidesData,
  theme: GlyphTheme,
): void {
  if (data.guides.length === 0) return
  const reach = viewport.width + viewport.height + 1000

  ctx.save()
  ctx.lineWidth = 1
  ctx.setLineDash([5, 4])
  ctx.font = `${LABEL_FONT_SIZE}px ${theme.fontFamily}`
  ctx.textAlign = 'left'
  ctx.textBaseline = 'bottom'

  for (const entry of data.guides) {
    const { guide } = entry
    const colour =
      entry.scope === 'master' ? theme.guideGlobal : theme.guideLocal
    const origin = viewport.toScreen({ x: guide.x, y: guide.y })
    const radians = (guide.angle * Math.PI) / 180
    // Font space is y-up, canvas space is y-down, so the angle flips.
    const dx = Math.cos(radians)
    const dy = -Math.sin(radians)

    ctx.strokeStyle = colour
    ctx.beginPath()
    ctx.moveTo(origin.x - dx * reach, origin.y - dy * reach)
    ctx.lineTo(origin.x + dx * reach, origin.y + dy * reach)
    ctx.stroke()

    if (data.showLabels && guide.name) {
      ctx.fillStyle = colour
      ctx.fillText(guide.name, origin.x + 4, origin.y - 3)
    }
  }
  ctx.restore()
}
