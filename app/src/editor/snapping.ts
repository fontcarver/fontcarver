/**
 * Snapping while dragging or drawing.
 *
 * Design spec §4.2: 6 CSS pixels by default, tried in the order node > handle > anchor >
 * guide > metric > grid > alignment line, and suspended entirely while Ctrl is
 * held. Point targets snap both axes; guides, metric lines, the grid and
 * alignment lines snap only the axis they constrain, so dragging along a
 * baseline stays free in x.
 *
 * Which families are live is a preference (`SnapPrefs`), and so is the grid
 * step, which is why the grid only appears once `prefs.grid_size` is set: the
 * canvas draws exactly the grid the snap uses.
 */

import type { Anchor } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { SceneMetrics } from '../views/glyph/glyphScene'

import type { ScopedGuide } from './hitTest'
import type { EditOutline } from './outlineGeometry'

/** Snap distance in CSS pixels, design spec §4.2. */
export const SNAP_DISTANCE = 6

/** What a snap latched onto. */
export interface SnapMatch {
  /** Adjusted delta. */
  delta: Point2
  /** Human name of the target, for the HUD; `null` when nothing snapped. */
  label: string | null
  /** Font-space x an alignment line was drawn at, when one matched. */
  alignedX: number | null
  /** Font-space y an alignment line was drawn at, when one matched. */
  alignedY: number | null
  /** The snapped point, for the halo. */
  point: Point2 | null
}

/**
 * What a snap target is called in the HUD.
 *
 * The names are handed in already translated: this module has no i18n of its
 * own, and every user-visible string in the editor comes from one bundle.
 */
export interface SnapNames {
  node: string
  handle: string
  /** Fallback for a guide with no name of its own. */
  guide: string
  anchor: string
  grid: string
  component: string
  background: string
  zone: string
  advance: string
  baseline: string
  xHeight: string
  capHeight: string
  ascender: string
  descender: string
}

/** Which snap families are live, and how coarse the grid is. */
export interface SnapSettings {
  nodes: boolean
  handles: boolean
  anchors: boolean
  guides: boolean
  metrics: boolean
  grid: boolean
  components: boolean
  zones: boolean
  background: boolean
  /** Snap reach in CSS pixels; zero disables snapping. */
  snapDistance: number
  /** Node hit radius in CSS pixels; other hit families scale proportionally. */
  hitDistance: number
  /** Grid step in font units; zero disables the grid entirely. */
  gridSize: number
}

/** Everything on until the kernel's preferences arrive. */
export const DEFAULT_SNAP_SETTINGS: SnapSettings = {
  nodes: true,
  handles: true,
  anchors: true,
  guides: true,
  metrics: true,
  grid: false,
  components: true,
  zones: true,
  background: true,
  snapDistance: SNAP_DISTANCE,
  hitDistance: 5,
  gridSize: 0,
}

/** Everything snapping looks at. */
export interface SnapInput {
  outline: EditOutline
  anchors: readonly Anchor[]
  guides: readonly ScopedGuide[]
  metrics: SceneMetrics
  /** Ghost nodes of the layer's components, from `glyphs.flattened_components`. */
  componentPoints: readonly Point2[]
  /** Both edges of each active master's alignment zone. */
  zoneLines?: readonly number[]
  /** Outline points from the active master's background layer. */
  backgroundPoints?: readonly Point2[]
  names: SnapNames
  viewport: Viewport
  settings: SnapSettings
  /** `contour:point` keys excluded from snapping, i.e. what is being dragged. */
  exclude: ReadonlySet<string>
}

interface PointTarget {
  point: Point2
  label: string
  rank: number
}

/** Nothing snapped. */
export const NO_SNAP: SnapMatch = {
  delta: { x: 0, y: 0 },
  label: null,
  alignedX: null,
  alignedY: null,
  point: null,
}

/** The unsnapped result for a delta. */
function unsnapped(delta: Point2): SnapMatch {
  return {
    delta,
    label: null,
    alignedX: null,
    alignedY: null,
    point: null,
  }
}

function pointTargets(input: SnapInput): PointTarget[] {
  const targets: PointTarget[] = []
  for (const point of input.outline.points) {
    if (!(point.onCurve ? input.settings.nodes : input.settings.handles))
      continue
    const key = `${point.ref.contour}:${point.ref.point}`
    if (input.exclude.has(key)) continue
    targets.push({
      point: { x: point.x, y: point.y },
      label: point.onCurve ? input.names.node : input.names.handle,
      rank: point.onCurve ? 0 : 1,
    })
  }
  if (input.settings.anchors)
    for (const anchor of input.anchors)
      targets.push({
        point: { x: anchor.x, y: anchor.y },
        label: anchor.name || input.names.anchor,
        rank: 2,
      })
  // §4.2 does not name the component ghosts in its order, so they sit just
  // behind the layer's own features: a ghost is a hint about a neighbouring
  // shape, never a reason to miss a node of the glyph being drawn.
  if (input.settings.components)
    for (const point of input.componentPoints)
      targets.push({ point, label: input.names.component, rank: 3 })
  if (input.settings.background)
    for (const point of input.backgroundPoints ?? [])
      targets.push({ point, label: input.names.background, rank: 4 })
  return targets
}

/** Horizontal metric lines, in font units, with their labels. */
function metricLines(input: SnapInput): { y: number; label: string }[] {
  const { metrics, names } = input
  return [
    { y: 0, label: names.baseline },
    { y: metrics.xHeight, label: names.xHeight },
    { y: metrics.capHeight, label: names.capHeight },
    { y: metrics.ascender, label: names.ascender },
    { y: metrics.descender, label: names.descender },
  ]
}

/**
 * Snaps a proposed delta.
 *
 * `moved` are the points the gesture is dragging, already offset by `delta`;
 * the first is the one under the pointer and the one snapping aims at.
 */
export function snapDelta(
  input: SnapInput,
  delta: Point2,
  moved: readonly Point2[],
): SnapMatch {
  const primary = moved[0]
  if (!primary) return unsnapped(delta)
  const scale = input.viewport.scale > 0 ? input.viewport.scale : 1
  const reach = input.settings.snapDistance / scale
  if (!Number.isFinite(reach) || reach <= 0) return unsnapped(delta)

  let best: { target: PointTarget; distance: number } | null = null
  for (const target of pointTargets(input)) {
    const distance = Math.hypot(
      target.point.x - primary.x,
      target.point.y - primary.y,
    )
    if (distance > reach) continue
    if (
      !best ||
      target.rank < best.target.rank ||
      (target.rank === best.target.rank && distance < best.distance)
    )
      best = { target, distance }
  }
  if (best)
    return {
      delta: {
        x: delta.x + (best.target.point.x - primary.x),
        y: delta.y + (best.target.point.y - primary.y),
      },
      label: best.target.label,
      alignedX: null,
      alignedY: null,
      point: best.target.point,
    }

  // Axis-constrained targets: guides, then metric lines, then alignment lines.
  let snappedX: { value: number; label: string } | null = null
  let snappedY: { value: number; label: string } | null = null
  let diagonal: { point: Point2; label: string; distance: number } | null = null

  if (input.settings.guides)
    for (const entry of input.guides) {
      const { guide } = entry
      const angle = ((guide.angle % 180) + 180) % 180
      const name = guide.name ?? input.names.guide
      if (angle === 0 && Math.abs(guide.y - primary.y) <= reach && !snappedY)
        snappedY = { value: guide.y, label: name }
      if (angle === 90 && Math.abs(guide.x - primary.x) <= reach && !snappedX)
        snappedX = { value: guide.x, label: name }
      if (angle !== 0 && angle !== 90) {
        const radians = (angle * Math.PI) / 180
        const nx = -Math.sin(radians)
        const ny = Math.cos(radians)
        const offset = (primary.x - guide.x) * nx + (primary.y - guide.y) * ny
        const distance = Math.abs(offset)
        if (distance <= reach && (!diagonal || distance < diagonal.distance))
          diagonal = {
            point: { x: primary.x - offset * nx, y: primary.y - offset * ny },
            label: name,
            distance,
          }
      }
    }

  // Project both coordinates together: a lower-priority axis snap must not
  // pull the result back off the oblique guide.
  if (diagonal && !snappedX && !snappedY)
    return {
      delta: {
        x: delta.x + diagonal.point.x - primary.x,
        y: delta.y + diagonal.point.y - primary.y,
      },
      point: diagonal.point,
      label: diagonal.label,
      alignedX: null,
      alignedY: null,
    }

  if (!snappedY && input.settings.metrics)
    for (const line of metricLines(input))
      if (Math.abs(line.y - primary.y) <= reach) {
        snappedY = { value: line.y, label: line.label }
        break
      }

  if (!snappedX && input.settings.metrics)
    for (const x of [0, input.metrics.width])
      if (Math.abs(x - primary.x) <= reach) {
        snappedX = { value: x, label: input.names.advance }
        break
      }

  if (!snappedY && input.settings.zones)
    for (const y of input.zoneLines ?? [])
      if (Math.abs(y - primary.y) <= reach) {
        snappedY = { value: y, label: input.names.zone }
        break
      }

  const step = input.settings.gridSize
  if (input.settings.grid && step > 0) {
    const nearest = (value: number) => Math.round(value / step) * step
    if (!snappedX && Math.abs(nearest(primary.x) - primary.x) <= reach)
      snappedX = { value: nearest(primary.x), label: input.names.grid }
    if (!snappedY && Math.abs(nearest(primary.y) - primary.y) <= reach)
      snappedY = { value: nearest(primary.y), label: input.names.grid }
  }

  let alignedX: number | null = null
  let alignedY: number | null = null
  if (
    (input.settings.nodes || input.settings.handles) &&
    (!snappedX || !snappedY)
  )
    for (const point of input.outline.points) {
      if (!(point.onCurve ? input.settings.nodes : input.settings.handles))
        continue
      const key = `${point.ref.contour}:${point.ref.point}`
      if (input.exclude.has(key)) continue
      if (!snappedX && Math.abs(point.x - primary.x) <= reach) {
        snappedX = { value: point.x, label: 'align' }
        alignedX = point.x
      }
      if (!snappedY && Math.abs(point.y - primary.y) <= reach) {
        snappedY = { value: point.y, label: 'align' }
        alignedY = point.y
      }
      if (snappedX && snappedY) break
    }

  if (!snappedX && !snappedY) return unsnapped(delta)
  const next = {
    x: snappedX ? delta.x + (snappedX.value - primary.x) : delta.x,
    y: snappedY ? delta.y + (snappedY.value - primary.y) : delta.y,
  }
  const labels = [snappedY?.label, snappedX?.label].filter(
    (label): label is string => label !== undefined && label !== 'align',
  )
  return {
    delta: next,
    label: labels[0] ?? null,
    alignedX,
    alignedY,
    point: {
      x: snappedX ? snappedX.value : primary.x,
      y: snappedY ? snappedY.value : primary.y,
    },
  }
}
