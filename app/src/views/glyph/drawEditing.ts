/**
 * The editing overlays of the canvas (design spec §4.1 items 10 and 14).
 *
 * Everything here is transient: it exists because a gesture is in progress or
 * because the pointer is hovering something, and it disappears the moment the
 * user lets go. Keeping it in one module means the editor's feedback can be
 * reasoned about — and turned off — as a unit.
 */

import type { Point2, Viewport } from '../../canvas/Viewport'

import type { GlyphTheme } from './glyphTheme'

/** A marquee rectangle in canvas space. */
export interface MarqueeBox {
  x0: number
  y0: number
  x1: number
  y1: number
}

/** What the editing overlay draws this frame. */
export interface EditingOverlay {
  /** Rubber-band rectangle, in canvas space. */
  marquee: MarqueeBox | null
  /** Font-space point the gesture snapped to, drawn as a halo. */
  snap: Point2 | null
  /** Font-space x an alignment line runs along. */
  alignX: number | null
  /** Font-space y an alignment line runs along. */
  alignY: number | null
  /** Font-space point where a click would insert a node. */
  insertGhost: Point2 | null
  /** Font-space points of a nearby component, drawn as faint ghost nodes. */
  componentGhosts: readonly Point2[]
}

/** Nothing to draw. */
export const NO_OVERLAY: EditingOverlay = {
  marquee: null,
  snap: null,
  alignX: null,
  alignY: null,
  insertGhost: null,
  componentGhosts: [],
}

const SNAP_HALO_RADIUS = 5
const INSERT_GHOST_RADIUS = 2
const COMPONENT_GHOST_RADIUS = 1.5

/** Draws the transient editing feedback on top of the outline. */
export function drawEditing(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  overlay: EditingOverlay,
  theme: GlyphTheme,
): void {
  ctx.save()

  if (overlay.alignX !== null || overlay.alignY !== null) {
    ctx.globalAlpha = 0.6
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 1
    ctx.beginPath()
    if (overlay.alignX !== null) {
      const x = viewport.toScreen({ x: overlay.alignX, y: 0 }).x
      ctx.moveTo(x, 0)
      ctx.lineTo(x, viewport.height)
    }
    if (overlay.alignY !== null) {
      const y = viewport.toScreen({ x: 0, y: overlay.alignY }).y
      ctx.moveTo(0, y)
      ctx.lineTo(viewport.width, y)
    }
    ctx.stroke()
    ctx.globalAlpha = 1
  }

  if (overlay.componentGhosts.length > 0) {
    ctx.globalAlpha = 0.6
    ctx.fillStyle = theme.handle
    for (const ghost of overlay.componentGhosts) {
      const at = viewport.toScreen(ghost)
      ctx.beginPath()
      ctx.arc(at.x, at.y, COMPONENT_GHOST_RADIUS, 0, Math.PI * 2)
      ctx.fill()
    }
    ctx.globalAlpha = 1
  }

  if (overlay.insertGhost) {
    const at = viewport.toScreen(overlay.insertGhost)
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.arc(at.x, at.y, INSERT_GHOST_RADIUS, 0, Math.PI * 2)
    ctx.stroke()
  }

  if (overlay.snap) {
    const at = viewport.toScreen(overlay.snap)
    ctx.globalAlpha = 0.25
    ctx.fillStyle = theme.accent
    ctx.beginPath()
    ctx.arc(at.x, at.y, SNAP_HALO_RADIUS, 0, Math.PI * 2)
    ctx.fill()
    ctx.globalAlpha = 1
  }

  if (overlay.marquee) {
    const { x0, y0, x1, y1 } = overlay.marquee
    ctx.globalAlpha = 0.08
    ctx.fillStyle = theme.accent
    ctx.fillRect(x0, y0, x1 - x0, y1 - y0)
    ctx.globalAlpha = 1
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 1
    ctx.setLineDash([4, 3])
    ctx.strokeRect(x0 + 0.5, y0 + 0.5, x1 - x0, y1 - y0)
    ctx.setLineDash([])
  }

  ctx.restore()
}
