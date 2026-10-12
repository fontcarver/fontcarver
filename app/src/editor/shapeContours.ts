/**
 * The rectangle and the ellipse the shape tools draw (design spec §5).
 *
 * A shape is built as geometry and handed to `pathToContour`, so the same
 * construction serves the drag, the numeric dialog and the tests. The curve
 * flavour decides both the representation — four cubic quarters on a
 * PostScript layer, eight quadratic eighths on a TrueType one — and the
 * winding, because a TrueType outer contour runs clockwise and a PostScript
 * one counter-clockwise.
 */

import type { Contour } from '../api'
import type { FontBox, Point2 } from '../canvas/Viewport'

import {
  pathToContour,
  withWinding,
  type GeometricPath,
  type LayerCurves,
  type PathSegment,
} from './pathGeometry'

/** The circular-arc constant for a cubic quarter turn. */
export const KAPPA = 0.5522847

/** How far a quadratic control sits outside a 45° arc, as a radius factor. */
export const QUAD_ARC_FACTOR = 1 / Math.cos(Math.PI / 8)

/** Which shape a drag or a dialog is creating. */
export type ShapeKind = 'rectangle' | 'ellipse'

/** The rectangle of a box, counter-clockwise in font space. */
export function rectanglePath(box: FontBox): GeometricPath {
  const corners: Point2[] = [
    { x: box.x0, y: box.y0 },
    { x: box.x1, y: box.y0 },
    { x: box.x1, y: box.y1 },
    { x: box.x0, y: box.y1 },
  ]
  const segments: PathSegment[] = corners.map((corner, index) => ({
    kind: 'line',
    from: corner,
    to: corners[(index + 1) % corners.length] as Point2,
    controls: [],
  }))
  return { closed: true, start: corners[0] as Point2, segments }
}

/** The ellipse inscribed in a box, counter-clockwise in font space. */
export function ellipsePath(box: FontBox, curves: LayerCurves): GeometricPath {
  const cx = (box.x0 + box.x1) / 2
  const cy = (box.y0 + box.y1) / 2
  const rx = (box.x1 - box.x0) / 2
  const ry = (box.y1 - box.y0) / 2
  const on = (angle: number, radius = 1): Point2 => ({
    x: cx + Math.cos(angle) * rx * radius,
    y: cy + Math.sin(angle) * ry * radius,
  })

  if (curves === 'quadratic') {
    // Eight 45° arcs: the control of each sits on the bisector, one
    // sec(22.5°) out, which is the exact quadratic fit for that sweep.
    const step = Math.PI / 4
    const segments: PathSegment[] = []
    for (let index = 0; index < 8; index += 1) {
      const from = on(index * step)
      const to = on(((index + 1) % 8) * step)
      segments.push({
        kind: 'quad',
        from,
        to,
        controls: [on(index * step + step / 2, QUAD_ARC_FACTOR)],
      })
    }
    return { closed: true, start: segments[0]?.from ?? on(0), segments }
  }

  // Four cubic quarters with the classic κ control offset.
  const quarter = Math.PI / 2
  const segments: PathSegment[] = []
  for (let index = 0; index < 4; index += 1) {
    const a = index * quarter
    const b = ((index + 1) % 4) * quarter
    const from = on(a)
    const to = on(b)
    // The tangent at an axis point runs along the other axis.
    const tangent = (angle: number): Point2 => ({
      x: -Math.sin(angle) * rx * KAPPA,
      y: Math.cos(angle) * ry * KAPPA,
    })
    const t0 = tangent(a)
    const t1 = tangent(b)
    segments.push({
      kind: 'cubic',
      from,
      to,
      controls: [
        { x: from.x + t0.x, y: from.y + t0.y },
        { x: to.x - t1.x, y: to.y - t1.y },
      ],
    })
  }
  return { closed: true, start: segments[0]?.from ?? on(0), segments }
}

/** The contour a shape tool commits for one box. */
export function shapeContour(
  kind: ShapeKind,
  box: FontBox,
  curves: LayerCurves,
): Contour {
  const path =
    kind === 'rectangle' ? rectanglePath(box) : ellipsePath(box, curves)
  // Every on-curve point of an ellipse is an extremum, so it is smooth; a
  // rectangle's corners are corners.
  return pathToContour(withWinding(path, curves), () => kind === 'ellipse')
}

/** Whether a box has any extent at all; a zero-area shape is not committed. */
export function boxIsEmpty(box: FontBox): boolean {
  return Math.abs(box.x1 - box.x0) < 1e-6 || Math.abs(box.y1 - box.y0) < 1e-6
}
