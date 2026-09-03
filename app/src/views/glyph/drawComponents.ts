/**
 * Component drawing.
 *
 * The kernel returns one component-flattened path per layer, which already
 * resolves nested components and their transforms. Rather than re-resolving
 * base glyphs in the browser, the flattened path is drawn first in the muted
 * component colour and the layer's own contours are drawn over it, so what
 * remains visible in that colour is exactly the component contribution. Each
 * component also gets a marker and a label at its own origin.
 */

import type { Component } from '../../api'
import type { Viewport } from '../../canvas/Viewport'

import type { GlyphTheme } from './glyphTheme'

const MARKER_SIZE = 5
const LABEL_FONT_SIZE = 10
const LABEL_GAP = 2

interface LabelBox {
  left: number
  top: number
  right: number
  bottom: number
}

function intersects(a: LabelBox, b: LabelBox): boolean {
  return (
    a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
  )
}

function labelPosition(
  x: number,
  preferredY: number,
  width: number,
  placed: readonly LabelBox[],
): { y: number; box: LabelBox } {
  const step = LABEL_FONT_SIZE + LABEL_GAP
  for (let attempt = 0; attempt <= placed.length * 2 + 2; attempt += 1) {
    const distance = Math.ceil(attempt / 2) * step
    const y =
      attempt === 0
        ? preferredY
        : preferredY + (attempt % 2 === 1 ? -distance : distance)
    const box = {
      left: x,
      top: y - LABEL_FONT_SIZE,
      right: x + width,
      bottom: y,
    }
    if (!placed.some((other) => intersects(box, other))) return { y, box }
  }
  const y = preferredY - (placed.length + 1) * step
  return {
    y,
    box: {
      left: x,
      top: y - LABEL_FONT_SIZE,
      right: x + width,
      bottom: y,
    },
  }
}

/** What `drawComponents` renders. */
export interface ComponentsData {
  /** Component-flattened outline of the whole layer, in font units. */
  path: Path2D | null
  components: readonly Component[]
  /** Fill instead of stroking, matching the outline preview mode. */
  filled: boolean
  /** Draw the per-component origin markers and names. */
  showLabels: boolean
}

/** Draws the component contribution of one layer. */
export function drawComponents(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: ComponentsData,
  theme: GlyphTheme,
): void {
  if (data.components.length === 0) return

  if (data.path) {
    ctx.save()
    ctx.translate(viewport.offset.x, viewport.offset.y)
    ctx.scale(viewport.scale, -viewport.scale)
    if (data.filled) {
      ctx.fillStyle = theme.component
      ctx.fill(data.path, 'nonzero')
    } else {
      ctx.strokeStyle = theme.component
      ctx.lineWidth = 1 / viewport.scale
      ctx.stroke(data.path)
    }
    ctx.restore()
  }

  if (!data.showLabels) return

  ctx.save()
  ctx.strokeStyle = theme.component
  ctx.fillStyle = theme.component
  ctx.lineWidth = 1
  ctx.font = `${LABEL_FONT_SIZE}px ${theme.fontFamily}`
  ctx.textAlign = 'left'
  ctx.textBaseline = 'bottom'
  const labels: LabelBox[] = []
  for (const component of data.components) {
    const origin = viewport.toScreen({
      x: component.transform[4],
      y: component.transform[5],
    })
    ctx.beginPath()
    ctx.moveTo(origin.x - MARKER_SIZE, origin.y)
    ctx.lineTo(origin.x + MARKER_SIZE, origin.y)
    ctx.moveTo(origin.x, origin.y - MARKER_SIZE)
    ctx.lineTo(origin.x, origin.y + MARKER_SIZE)
    ctx.stroke()
    const text = component.base.toString()
    const x = origin.x + MARKER_SIZE + LABEL_GAP
    const position = labelPosition(
      x,
      origin.y - LABEL_GAP,
      ctx.measureText(text).width,
      labels,
    )
    labels.push(position.box)
    ctx.fillText(text, x, position.y)
  }
  ctx.restore()
}
