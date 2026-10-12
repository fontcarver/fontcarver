/**
 * Guide drawing.
 *
 * A guide is a point and an angle in font units, so it is an infinite line;
 * it is clipped to the canvas by extending it far enough in both directions.
 * Master guides are font-wide and drawn in the global colour, layer guides in
 * the local one, following the convention every incumbent editor uses.
 */

import type { Contour } from '../../api'
import type { Viewport } from '../../canvas/Viewport'

import type { SceneGuide } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'
import { guideMeasurements } from './guideMeasure'
import { measurementGuideKey, type GuideOwner } from './guideIdentity'

const LABEL_FONT_SIZE = 10

/** Radius of the angle knob a selected guide carries (§4.1 item 12). */
const KNOB_RADIUS = 3

/** How far along the guide the knob sits, in CSS pixels. */
const KNOB_REACH = 48

/** What `drawGuides` renders. */
export interface GuidesData {
  guides: readonly SceneGuide[]
  showLabels: boolean
  /** Selection keys of the selected guides (`guide:scope:index`). */
  selected?: ReadonlySet<string>
  /** Owner-scoped keys of the guides shown as measurement guides. */
  measurement?: ReadonlySet<string>
  /** Ownership used to match session-only measurement markers. */
  owner?: GuideOwner
  /** The outline the measurement guides are measured against. */
  contours?: readonly Contour[]
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

  data.guides.forEach((entry) => {
    const { guide } = entry
    const key = `${entry.scope}:${entry.index}`
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

    // Display-only angle marker for the selected guide. Angle editing is
    // currently numeric in the Guides panel; pointer interaction is deferred.
    if (data.selected?.has(`guide:${key}`)) {
      ctx.setLineDash([])
      ctx.fillStyle = colour
      ctx.beginPath()
      ctx.arc(
        origin.x + dx * KNOB_REACH,
        origin.y + dy * KNOB_REACH,
        KNOB_RADIUS,
        0,
        Math.PI * 2,
      )
      ctx.fill()
      ctx.setLineDash([5, 4])
    }

    const measurementKey = data.owner
      ? measurementGuideKey(data.owner, entry.scope, entry.index)
      : key
    if (!data.measurement?.has(measurementKey) || !data.contours) return
    ctx.setLineDash([])
    ctx.fillStyle = colour
    ctx.textAlign = 'center'
    ctx.textBaseline = 'bottom'
    for (const span of guideMeasurements(data.contours, guide)) {
      const from = viewport.toScreen(span.from)
      const to = viewport.toScreen(span.to)
      ctx.fillText(
        String(Math.round(span.length)),
        (from.x + to.x) / 2,
        (from.y + to.y) / 2 - 2,
      )
    }
    ctx.textAlign = 'left'
    ctx.setLineDash([5, 4])
  })
  ctx.restore()
}
