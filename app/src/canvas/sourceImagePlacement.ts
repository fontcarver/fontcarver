/**
 * On-canvas placement handles for a layer's source image.
 *
 * The stored placement is the six-coefficient affine that maps the image's own
 * y-up pixel frame — `[0, width] × [0, height]`, the frame `drawSourceImage`
 * paints in — into font units. Every handle is expressed in that frame, so a
 * corner stays on its corner however the image is already rotated or skewed.
 *
 * Nothing here touches React or the kernel: a gesture takes a transform and a
 * pointer position and returns the next transform, which is what makes the
 * numeric fields in the inspector and a drag on the canvas the same operation.
 */

import {
  about,
  applyAffine,
  composeAffine,
  invertAffine,
  isInvertible,
  rotateAbout,
  scaleAbout,
  type Affine,
} from '../editor/affine'

import type { Point2, Viewport } from './Viewport'
import type { SourceImageDraw } from './sourceImage'

/** What one handle changes. */
export type PlacementHandle =
  | 'move'
  | 'scale-sw'
  | 'scale-se'
  | 'scale-ne'
  | 'scale-nw'
  | 'skew-s'
  | 'skew-e'
  | 'skew-n'
  | 'skew-w'
  | 'rotate'

/** A handle's position in the image's own pixel frame and on the canvas. */
export interface PlacementMarker {
  handle: PlacementHandle
  /** Position in the image's y-up pixel frame. */
  local: Point2
  /** Position in font units. */
  font: Point2
  /** Position in CSS pixels on the canvas. */
  screen: Point2
}

/** Hit radius of a placement handle, in CSS pixels. */
export const PLACEMENT_HIT = 7
/** Distance of the rotation handle beyond the top edge, in CSS pixels. */
const ROTATE_REACH = 26
/** Drawn half-size of a square handle, in CSS pixels. */
const HANDLE_SIZE = 4

/** The image's four corners in its own pixel frame, counter-clockwise from the origin. */
function corners(image: SourceImageDraw): Point2[] {
  return [
    { x: 0, y: 0 },
    { x: image.width, y: 0 },
    { x: image.width, y: image.height },
    { x: 0, y: image.height },
  ]
}

/** Every handle of one placed image, in drawing order. */
export function placementMarkers(
  viewport: Viewport,
  image: SourceImageDraw,
): PlacementMarker[] {
  const { width, height, transform } = image
  if (
    width <= 0 ||
    height <= 0 ||
    !transform.every(Number.isFinite) ||
    !isInvertible(transform)
  )
    return []
  const place = (handle: PlacementHandle, local: Point2): PlacementMarker => {
    const font = applyAffine(transform, local)
    return { handle, local, font, screen: viewport.toScreen(font) }
  }
  const markers: PlacementMarker[] = [
    place('scale-sw', { x: 0, y: 0 }),
    place('scale-se', { x: width, y: 0 }),
    place('scale-ne', { x: width, y: height }),
    place('scale-nw', { x: 0, y: height }),
    place('skew-s', { x: width / 2, y: 0 }),
    place('skew-e', { x: width, y: height / 2 }),
    place('skew-n', { x: width / 2, y: height }),
    place('skew-w', { x: 0, y: height / 2 }),
  ]
  // The rotation handle sits beyond the top edge, measured on the canvas so it
  // stays the same distance away at every zoom.
  const top = applyAffine(transform, { x: width / 2, y: height })
  const inner = applyAffine(transform, { x: width / 2, y: height / 2 })
  const topScreen = viewport.toScreen(top)
  const innerScreen = viewport.toScreen(inner)
  const away = {
    x: topScreen.x - innerScreen.x,
    y: topScreen.y - innerScreen.y,
  }
  const length = Math.hypot(away.x, away.y)
  const screen =
    length > 1e-6
      ? {
          x: topScreen.x + (away.x / length) * ROTATE_REACH,
          y: topScreen.y + (away.y / length) * ROTATE_REACH,
        }
      : { x: topScreen.x, y: topScreen.y - ROTATE_REACH }
  const font = viewport.toFont(screen)
  markers.push({
    handle: 'rotate',
    local: applyAffine(invertAffine(transform), font),
    font,
    screen,
  })
  return markers
}

/** The handle under a canvas position, or `move` inside the image, else `null`. */
export function placementHandleAt(
  viewport: Viewport,
  image: SourceImageDraw,
  at: Point2,
): PlacementHandle | null {
  const markers = placementMarkers(viewport, image)
  if (markers.length === 0) return null
  let best: { handle: PlacementHandle; distance: number } | null = null
  for (const marker of markers) {
    const distance = Math.hypot(marker.screen.x - at.x, marker.screen.y - at.y)
    if (distance <= PLACEMENT_HIT && (!best || distance < best.distance))
      best = { handle: marker.handle, distance }
  }
  if (best) return best.handle
  const local = applyAffine(invertAffine(image.transform), viewport.toFont(at))
  return local.x >= 0 &&
    local.y >= 0 &&
    local.x <= image.width &&
    local.y <= image.height
    ? 'move'
    : null
}

/** The CSS cursor for one handle. */
export function placementCursor(handle: PlacementHandle | null): string {
  switch (handle) {
    case 'move':
      return 'move'
    case 'rotate':
      return 'crosshair'
    case null:
      return 'default'
    default:
      return 'nwse-resize'
  }
}

/** A gesture in progress on one handle. */
export interface PlacementGesture {
  handle: PlacementHandle
  /** Placement when the press happened. */
  start: Affine
  /** Font-space position of the press. */
  from: Point2
  /** Handle position in the image's pixel frame when the press happened. */
  local: Point2
}

/** Starts a gesture, or `null` when the handle is not on this image. */
export function beginPlacement(
  viewport: Viewport,
  image: SourceImageDraw,
  at: Point2,
): PlacementGesture | null {
  const handle = placementHandleAt(viewport, image, at)
  if (!handle) return null
  const marker = placementMarkers(viewport, image).find(
    (entry) => entry.handle === handle,
  )
  return {
    handle,
    start: [...image.transform] as Affine,
    from: viewport.toFont(at),
    local: marker?.local ?? { x: 0, y: 0 },
  }
}

/** Opposite corner or edge of a handle, in the image's pixel frame. */
function pivotOf(
  handle: PlacementHandle,
  width: number,
  height: number,
): Point2 {
  switch (handle) {
    case 'scale-sw':
      return { x: width, y: height }
    case 'scale-se':
      return { x: 0, y: height }
    case 'scale-ne':
      return { x: 0, y: 0 }
    case 'scale-nw':
      return { x: width, y: 0 }
    case 'skew-s':
      return { x: width / 2, y: height }
    case 'skew-n':
      return { x: width / 2, y: 0 }
    case 'skew-e':
      return { x: 0, y: height / 2 }
    case 'skew-w':
      return { x: width, y: height / 2 }
    default:
      return { x: width / 2, y: height / 2 }
  }
}

/**
 * The placement a gesture has reached, or the starting placement when the
 * pointer has not moved anywhere usable.
 *
 * `uniform` constrains a corner to one scale factor and snaps a rotation to 15°;
 * it is what the Shift modifier means for this gesture.
 */
export function updatePlacement(
  gesture: PlacementGesture,
  image: { width: number; height: number },
  to: Point2,
  uniform: boolean,
): Affine {
  const { handle, start, from, local } = gesture
  const { width, height } = image
  if (!Number.isFinite(to.x) || !Number.isFinite(to.y)) return start
  if (handle === 'move') {
    const next: Affine = [...start]
    next[4] += to.x - from.x
    next[5] += to.y - from.y
    return next
  }
  if (handle === 'rotate') {
    const centre = applyAffine(start, { x: width / 2, y: height / 2 })
    const before = Math.atan2(from.y - centre.y, from.x - centre.x)
    const after = Math.atan2(to.y - centre.y, to.x - centre.x)
    let radians = after - before
    if (uniform) {
      const step = Math.PI / 12
      radians = Math.round(radians / step) * step
    }
    return finite(composeAffine(rotateAbout(centre, radians), start), start)
  }
  const inverse = invertAffine(start)
  const target = applyAffine(inverse, to)
  const pivot = pivotOf(handle, width, height)
  if (handle.startsWith('scale-')) {
    const spanX = local.x - pivot.x
    const spanY = local.y - pivot.y
    if (Math.abs(spanX) < 1e-9 || Math.abs(spanY) < 1e-9) return start
    let sx = (target.x - pivot.x) / spanX
    let sy = (target.y - pivot.y) / spanY
    if (uniform) {
      const magnitude = Math.max(Math.abs(sx), Math.abs(sy))
      sx = Math.sign(sx || 1) * magnitude
      sy = Math.sign(sy || 1) * magnitude
    }
    if (Math.abs(sx) < 1e-6 || Math.abs(sy) < 1e-6) return start
    return finite(composeAffine(start, scaleAbout(pivot, sx, sy)), start)
  }
  // A skew slides one edge along itself, keeping the opposite edge in place.
  const horizontal = handle === 'skew-n' || handle === 'skew-s'
  const reach = horizontal ? local.y - pivot.y : local.x - pivot.x
  if (Math.abs(reach) < 1e-9) return start
  const shift = horizontal ? target.x - local.x : target.y - local.y
  const factor = shift / reach
  if (!Number.isFinite(factor)) return start
  const linear: Affine = horizontal
    ? [1, 0, factor, 1, 0, 0]
    : [1, factor, 0, 1, 0, 0]
  const anchor = horizontal ? { x: 0, y: pivot.y } : { x: pivot.x, y: 0 }
  return finite(composeAffine(start, about(anchor, linear)), start)
}

/** Keeps a gesture from publishing a transform the kernel would refuse. */
function finite(candidate: Affine, fallback: Affine): Affine {
  return candidate.every(Number.isFinite) && isInvertible(candidate)
    ? candidate
    : fallback
}

/** Draws the placement frame and its handles over the painted image. */
export function drawPlacement(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  image: SourceImageDraw,
  colors: { frame: string; handle: string },
  hovered: PlacementHandle | null,
): void {
  const markers = placementMarkers(viewport, image)
  if (markers.length === 0) return
  const outline = corners(image).map((local) =>
    viewport.toScreen(applyAffine(image.transform, local)),
  )
  ctx.save()
  ctx.lineWidth = 1
  ctx.strokeStyle = colors.frame
  ctx.setLineDash([4, 3])
  ctx.beginPath()
  outline.forEach((point, index) =>
    index === 0 ? ctx.moveTo(point.x, point.y) : ctx.lineTo(point.x, point.y),
  )
  ctx.closePath()
  ctx.stroke()
  ctx.setLineDash([])
  const rotate = markers.find((marker) => marker.handle === 'rotate')
  const top = markers.find((marker) => marker.handle === 'skew-n')
  if (rotate && top) {
    ctx.beginPath()
    ctx.moveTo(top.screen.x, top.screen.y)
    ctx.lineTo(rotate.screen.x, rotate.screen.y)
    ctx.stroke()
  }
  for (const marker of markers) {
    const active = marker.handle === hovered
    ctx.fillStyle = active ? colors.handle : colors.frame
    ctx.strokeStyle = colors.handle
    if (marker.handle === 'rotate') {
      ctx.beginPath()
      ctx.arc(marker.screen.x, marker.screen.y, HANDLE_SIZE, 0, Math.PI * 2)
      ctx.fill()
      ctx.stroke()
      continue
    }
    const size = marker.handle.startsWith('skew-')
      ? HANDLE_SIZE - 1
      : HANDLE_SIZE
    ctx.beginPath()
    ctx.rect(marker.screen.x - size, marker.screen.y - size, size * 2, size * 2)
    ctx.fill()
    ctx.stroke()
  }
  ctx.restore()
}
