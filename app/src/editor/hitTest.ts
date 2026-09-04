/**
 * What the pointer is over.
 *
 * Tolerances are the design spec's (§4.2) and are expressed in CSS pixels, so
 * a target stays equally easy to grab at any zoom and on any display: the
 * radius is converted to font units through the viewport scale, which is
 * independent of the device pixel ratio (the backing store is denser, the
 * pointer coordinates are not). Precedence is fixed — selected node > node >
 * handle > anchor > guide > segment > component — so a click never surprises
 * the user by picking the thing behind the thing they aimed at.
 */

import type { Anchor, Guide } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'

import { nearestOnSegment, type EditOutline } from './outlineGeometry'
import { isSelected, type EditTarget, type GuideScope, type Selection } from './selection'

/** Hit radii in CSS pixels, design spec §4.2. */
export const HIT_RADII = {
  node: 5,
  handle: 5,
  segment: 4,
  anchor: 6,
  guide: 4,
  component: 4,
} as const

/** A guide together with the scope it came from. */
export interface ScopedGuide {
  guide: Guide
  scope: GuideScope
}

/** A component's flattened bounds, which is what the pointer can land on. */
export interface ComponentBounds {
  bbox: [number, number, number, number] | null
}

/** Everything the hit test looks at. */
export interface HitTestInput {
  outline: EditOutline
  anchors: readonly Anchor[]
  guides: readonly ScopedGuide[]
  components: readonly ComponentBounds[]
  viewport: Viewport
  selection: Selection
}

/** Modifiers that change what can be hit. */
export interface HitTestOptions {
  /** `Alt` ignores handles, both on hover and inside a marquee. */
  ignoreHandles?: boolean
}

/** What the pointer found. */
export interface Hit {
  target: EditTarget
  /** The exact font-space point on the target, for ghosts and snapping. */
  point: Point2
  /** Distance in CSS pixels. */
  distance: number
  /** Parametric position along a segment hit. */
  t?: number
}

function distance(a: Point2, b: Point2): number {
  return Math.hypot(a.x - b.x, a.y - b.y)
}

/** Perpendicular distance in CSS pixels from a screen point to a guide. */
function guideDistance(
  guide: Guide,
  viewport: Viewport,
  screen: Point2,
): number {
  const origin = viewport.toScreen({ x: guide.x, y: guide.y })
  const radians = (guide.angle * Math.PI) / 180
  // Font space is y-up, canvas space y-down, so the direction flips.
  const dx = Math.cos(radians)
  const dy = -Math.sin(radians)
  return Math.abs((screen.x - origin.x) * dy - (screen.y - origin.y) * dx)
}

function best(hits: readonly Hit[]): Hit | null {
  let winner: Hit | null = null
  for (const hit of hits)
    if (!winner || hit.distance < winner.distance) winner = hit
  return winner
}

/** Finds the topmost target under a canvas-space point, or `null`. */
export function hitTest(
  input: HitTestInput,
  screen: Point2,
  options: HitTestOptions = {},
): Hit | null {
  const { viewport, outline, selection } = input
  const font = viewport.toFont(screen)
  const scale = viewport.scale > 0 ? viewport.scale : 1
  const inPixels = (a: Point2): number => distance(a, font) * scale

  const selectedNodes: Hit[] = []
  const nodes: Hit[] = []
  const handles: Hit[] = []
  for (const point of outline.points) {
    const target: EditTarget = point.onCurve
      ? { kind: 'node', contour: point.ref.contour, point: point.ref.point }
      : { kind: 'handle', contour: point.ref.contour, point: point.ref.point }
    if (!point.onCurve && options.ignoreHandles) continue
    const away = inPixels(point)
    const radius = point.onCurve ? HIT_RADII.node : HIT_RADII.handle
    if (away > radius) continue
    const hit: Hit = { target, point: { x: point.x, y: point.y }, distance: away }
    if (!point.onCurve) handles.push(hit)
    else if (isSelected(selection, target)) selectedNodes.push(hit)
    else nodes.push(hit)
  }

  const anchors: Hit[] = []
  input.anchors.forEach((anchor, index) => {
    const away = inPixels(anchor)
    if (away <= HIT_RADII.anchor)
      anchors.push({
        target: { kind: 'anchor', index },
        point: { x: anchor.x, y: anchor.y },
        distance: away,
      })
  })

  const guides: Hit[] = []
  input.guides.forEach((entry, index) => {
    const away = guideDistance(entry.guide, viewport, screen)
    if (away <= HIT_RADII.guide)
      guides.push({
        target: { kind: 'guide', index, scope: entry.scope },
        point: { x: entry.guide.x, y: entry.guide.y },
        distance: away,
      })
  })

  const segments: Hit[] = []
  for (const segment of outline.segments) {
    const nearest = nearestOnSegment(segment, font)
    const away = nearest.distance * scale
    if (away <= HIT_RADII.segment)
      segments.push({
        target: {
          kind: 'segment',
          contour: segment.contour,
          segment: segment.index,
        },
        point: nearest.point,
        distance: away,
        t: nearest.t,
      })
  }

  const components: Hit[] = []
  const slack = HIT_RADII.component / scale
  input.components.forEach((component, index) => {
    const box = component.bbox
    if (!box) return
    const [x0, y0, x1, y1] = box
    if (
      font.x < x0 - slack ||
      font.x > x1 + slack ||
      font.y < y0 - slack ||
      font.y > y1 + slack
    )
      return
    components.push({
      target: { kind: 'component', index },
      point: { x: (x0 + x1) / 2, y: (y0 + y1) / 2 },
      distance: 0,
    })
  })

  return (
    best(selectedNodes) ??
    best(nodes) ??
    best(handles) ??
    best(anchors) ??
    best(guides) ??
    best(segments) ??
    best(components)
  )
}
