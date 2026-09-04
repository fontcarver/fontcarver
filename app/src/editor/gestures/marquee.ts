/**
 * Rubber-band selection.
 *
 * The rectangle is kept in canvas space because that is where the user drew it
 * and where the 1 px dashed outline is painted; every candidate is projected
 * into it. `Alt` ignores handles (design spec §4.3) so a marquee over a curve
 * can pick up only the nodes, and a component joins the selection only when it
 * is enclosed whole, which is what makes "sweep across the glyph" predictable.
 */

import type { Point2 } from '../../canvas/Viewport'
import type { HitTestInput } from '../hitTest'
import { selectionOf, type EditTarget, type Selection } from '../selection'

/** A normalised rectangle in canvas space. */
export interface MarqueeRect {
  x0: number
  y0: number
  x1: number
  y1: number
}

/** How a marquee behaves under the modifier keys. */
export interface MarqueeOptions {
  /** `Alt`: leave off-curve handles out of the selection. */
  ignoreHandles?: boolean
}

/** Normalises the two corners of a drag into a rectangle. */
export function marqueeRect(from: Point2, to: Point2): MarqueeRect {
  return {
    x0: Math.min(from.x, to.x),
    y0: Math.min(from.y, to.y),
    x1: Math.max(from.x, to.x),
    y1: Math.max(from.y, to.y),
  }
}

/** Width and height of a marquee in canvas pixels. */
export function marqueeSize(rect: MarqueeRect): { width: number; height: number } {
  return { width: rect.x1 - rect.x0, height: rect.y1 - rect.y0 }
}

function contains(rect: MarqueeRect, point: Point2): boolean {
  return (
    point.x >= rect.x0 &&
    point.x <= rect.x1 &&
    point.y >= rect.y0 &&
    point.y <= rect.y1
  )
}

/** Everything the rectangle encloses, as a selection. */
export function marqueeSelection(
  input: HitTestInput,
  rect: MarqueeRect,
  options: MarqueeOptions,
): Selection {
  const { viewport } = input
  const targets: EditTarget[] = []

  for (const point of input.outline.points) {
    if (!point.onCurve && options.ignoreHandles) continue
    if (!contains(rect, viewport.toScreen(point))) continue
    targets.push(
      point.onCurve
        ? { kind: 'node', contour: point.ref.contour, point: point.ref.point }
        : { kind: 'handle', contour: point.ref.contour, point: point.ref.point },
    )
  }

  input.anchors.forEach((anchor, index) => {
    if (contains(rect, viewport.toScreen(anchor)))
      targets.push({ kind: 'anchor', index })
  })

  input.components.forEach((component, index) => {
    const box = component.bbox
    if (!box) return
    const [x0, y0, x1, y1] = box
    const a = viewport.toScreen({ x: x0, y: y0 })
    const b = viewport.toScreen({ x: x1, y: y1 })
    if (contains(rect, a) && contains(rect, b))
      targets.push({ kind: 'component', index })
  })

  return selectionOf(targets)
}
