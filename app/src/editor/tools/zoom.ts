/**
 * The Zoom tool (design spec §5): click zooms in, Alt-click out, drag zooms to
 * the marquee. The wheel and the zoom commands share the same step so the view
 * behaves the same however the user reaches for it.
 */

import type { FontBox, Point2, Viewport } from '../../canvas/Viewport'

/** Zoom factor of one click or one wheel notch, design spec §4.2. */
export const ZOOM_FACTOR = 1.15

/** Zooms in (or out with Alt) around a canvas-space point. */
export function zoomAt(viewport: Viewport, at: Point2, out: boolean): void {
  viewport.zoomAt(at, out ? 1 / ZOOM_FACTOR : ZOOM_FACTOR)
}

/** Frames a canvas-space rectangle, which is what a zoom marquee asks for. */
export function zoomToRect(
  viewport: Viewport,
  rect: { x0: number; y0: number; x1: number; y1: number },
): void {
  const a = viewport.toFont({ x: rect.x0, y: rect.y0 })
  const b = viewport.toFont({ x: rect.x1, y: rect.y1 })
  const box: FontBox = {
    x0: Math.min(a.x, b.x),
    y0: Math.min(a.y, b.y),
    x1: Math.max(a.x, b.x),
    y1: Math.max(a.y, b.y),
  }
  viewport.fit(box, 8)
}

/** The smallest drag that counts as a zoom marquee rather than a click. */
export const ZOOM_MARQUEE_MIN = 6
