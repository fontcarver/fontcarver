/**
 * The Path menu's commands (design spec §11).
 *
 * `PathOp` is a whole-layer operation in the kernel — `fc-doc` hands the
 * layer's contours to `fc-geom` and takes the result back — so overlap
 * removal, extrema, simplification and stroke expansion always act on the
 * whole layer, whatever is selected. The four operations that *are*
 * selection-scoped (reverse, start point, join, break) address contours and
 * points directly, and their targets are worked out here from the selection.
 */

import type {
  Command,
  Contour,
  PathOp,
  PointRef,
  SimplifyOptions,
  StrokeOptions,
} from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import type { Selection } from '../../editor/selection'

/** `fw_geom::DEFAULT_SIMPLIFY_TOLERANCE`, the kernel's own default. */
export const DEFAULT_SIMPLIFY: SimplifyOptions = {
  error: 0.1,
  ignore_slopes: false,
  ignore_extrema: false,
  smooth_curves: false,
  force_lines: false,
  nearly_hv_lines: false,
  start_at_extrema: false,
}

/** `fw_doc::StrokeOptions::default()`, with a visible width for the dialog. */
export const DEFAULT_STROKE: StrokeOptions = {
  width: 40,
  nib: 'Round',
  cap: 'Round',
  join: 'Round',
  miter_limit: 4,
  remove_internal: false,
  remove_external: false,
}

/** Error a quadratic conversion is allowed, in font units. */
export const DEFAULT_QUADRATIC_ERROR = 0.5

/** One whole-layer geometry operation. */
export function pathOpCommand(target: LayerTarget, op: PathOp): Command {
  return { PathOp: { glyph: target.glyph, layer: target.layer, op } }
}

/**
 * "Tidy" is the canonical start point *and* the canonical contour order in one
 * undo entry, which is what the design spec's `Mod+⇧+T` promises; the kernel
 * exposes them as two operations because either is useful alone.
 */
export function tidyCommand(target: LayerTarget): Command {
  return {
    Batch: [
      pathOpCommand(target, 'CanonicalStart'),
      pathOpCommand(target, 'CanonicalOrder'),
    ],
  }
}

/** Correct winding and normalize start nodes and shape order in one undo step. */
export function correctDirectionCommand(target: LayerTarget): Command {
  return {
    Batch: [
      pathOpCommand(target, 'CorrectDirection'),
      pathOpCommand(target, 'CanonicalOrder'),
    ],
  }
}

/** `ReverseContours` for the listed contours. */
export function reverseCommand(
  target: LayerTarget,
  contours: readonly number[],
): Command {
  return {
    ReverseContours: {
      glyph: target.glyph,
      layer: target.layer,
      contours: [...contours],
    },
  }
}

/** `SetStartPoint` at one on-curve node. */
export function setStartPointCommand(
  target: LayerTarget,
  point: PointRef,
): Command {
  return { SetStartPoint: { glyph: target.glyph, layer: target.layer, point } }
}

/** `JoinContours` between two open ends. */
export function joinCommand(
  target: LayerTarget,
  a: PointRef,
  b: PointRef,
): Command {
  return { JoinContours: { glyph: target.glyph, layer: target.layer, a, b } }
}

/** `SplitContour` at one on-curve node: "Break". */
export function breakCommand(target: LayerTarget, point: PointRef): Command {
  return { SplitContour: { glyph: target.glyph, layer: target.layer, point } }
}

/**
 * Contours the selection touches, ascending; every contour of the layer when
 * nothing is selected, so a menu item with an empty canvas still means
 * something.
 */
export function scopedContours(
  contours: readonly Contour[],
  selection: Selection,
): number[] {
  const touched = new Set<number>()
  for (const target of selection.values())
    if (
      target.kind === 'node' ||
      target.kind === 'handle' ||
      target.kind === 'segment'
    )
      touched.add(target.contour)
  if (touched.size === 0) return contours.map((_, index) => index)
  return [...touched].sort((a, b) => a - b)
}

/** The single selected on-curve node, or `null` when it is not exactly one. */
export function onlyNode(selection: Selection): PointRef | null {
  let found: PointRef | null = null
  for (const target of selection.values()) {
    if (target.kind !== 'node') continue
    if (found) return null
    found = { contour: target.contour, point: target.point }
  }
  return found
}

/** A selected on-curve node whose closed contour supports changing its start. */
export function startPoint(
  contours: readonly Contour[],
  selection: Selection,
): PointRef | null {
  const node = onlyNode(selection)
  if (!node) return null
  const contour = contours[node.contour]
  const point = contour?.points[node.point]
  return contour?.closed && point && point.kind !== 'OffCurve' ? node : null
}

/** Whether a point index names an endpoint of an open contour. */
function endpointOf(
  contours: readonly Contour[],
  ref: PointRef,
): 'first' | 'last' | null {
  const contour = contours[ref.contour]
  if (!contour || contour.closed || contour.points.length === 0) return null
  if (ref.point === 0) return 'first'
  if (ref.point === contour.points.length - 1) return 'last'
  return null
}

/**
 * The two open ends a Join would connect.
 *
 * Prefer the last endpoint first to retain the user's path direction. The
 * kernel can reverse either contour and close the two ends of one contour.
 */
export function joinEnds(
  contours: readonly Contour[],
  selection: Selection,
): { a: PointRef; b: PointRef } | null {
  const ends: { ref: PointRef; end: 'first' | 'last' }[] = []
  for (const target of selection.values()) {
    if (target.kind !== 'node') continue
    const ref = { contour: target.contour, point: target.point }
    const end = endpointOf(contours, ref)
    if (!end) return null
    ends.push({ ref, end })
  }
  if (ends.length !== 2) return null
  const [first, second] = ends
  if (!first || !second) return null
  if (first.end === 'last') return { a: first.ref, b: second.ref }
  if (second.end === 'last') return { a: second.ref, b: first.ref }
  return { a: first.ref, b: second.ref }
}

/** Whether "Break" can act: one selected on-curve node with a contour to open. */
export function breakPoint(
  contours: readonly Contour[],
  selection: Selection,
): PointRef | null {
  const node = onlyNode(selection)
  if (!node) return null
  const contour = contours[node.contour]
  if (!contour) return null
  // An open contour splits only at an interior node; a closed one opens
  // anywhere. Both rules are the kernel's (`segments::split`).
  if (contour.closed) return node
  return node.point > 0 && node.point + 1 < contour.points.length ? node : null
}

/** `OpenCorner` at one node, with the setback it should use. */
export function openCornerCommand(
  target: LayerTarget,
  point: PointRef,
  depth: number,
): Command {
  return {
    OpenCorner: { glyph: target.glyph, layer: target.layer, point, depth },
  }
}

/** `ReconnectNodes` between the two nodes of one open corner. */
export function reconnectCommand(
  target: LayerTarget,
  a: PointRef,
  b: PointRef,
): Command {
  return {
    ReconnectNodes: { glyph: target.glyph, layer: target.layer, a, b },
  }
}

/** The on-curve stored indexes of one contour, in path order. */
function onCurve(contour: Contour): number[] {
  const found: number[] = []
  contour.points.forEach((point, index) => {
    if (point.kind !== 'OffCurve') found.push(index)
  })
  return found
}

/**
 * The default setback of an Open Corner, in font units.
 *
 * A corner has to stay recognisable after it is opened, so the mouth is a
 * small fraction of the shorter adjacent chord and never longer than one
 * fiftieth of the em — 20 units at 1000 upem, which is about the width of a
 * hairline and comfortably draggable at any sensible zoom.
 */
export function openCornerDepth(
  contour: Contour,
  point: number,
  upem: number,
): number {
  const order = onCurve(contour)
  const at = order.indexOf(point)
  if (at < 0 || order.length < 3) return 0
  const previous = at === 0 ? (contour.closed ? order.length - 1 : -1) : at - 1
  const next = at === order.length - 1 ? (contour.closed ? 0 : -1) : at + 1
  if (previous < 0 || next < 0) return 0
  const here = contour.points[point]
  const before = contour.points[order[previous] as number]
  const after = contour.points[order[next] as number]
  if (!here || !before || !after) return 0
  const chord = Math.min(
    Math.hypot(here.x - before.x, here.y - before.y),
    Math.hypot(after.x - here.x, after.y - here.y),
  )
  if (!Number.isFinite(chord) || chord <= 0) return 0
  const em = Number.isFinite(upem) && upem > 0 ? upem : 1000
  const wanted = Math.min(em / 50, chord / 4)
  return wanted >= 0.5 ? Math.round(wanted * 100) / 100 : 0
}

/** The corner an Open Corner would act on, with its setback, or `null`. */
export function openCornerPlan(
  contours: readonly Contour[],
  selection: Selection,
  upem: number,
): { point: PointRef; depth: number } | null {
  const node = onlyNode(selection)
  if (!node) return null
  const contour = contours[node.contour]
  const point = contour?.points[node.point]
  if (!contour || !point || point.kind === 'OffCurve') return null
  if (point.corner_radius) return null
  const depth = openCornerDepth(contour, node.point, upem)
  return depth > 0 ? { point: node, depth } : null
}

/**
 * The two nodes a Reconnect would merge, in path order.
 *
 * Exactly two selected on-curve nodes of one contour, one segment apart. The
 * selection order does not matter: whichever of the two the path reaches first
 * keeps its identity, because that is the node whose incoming segment survives.
 */
export function reconnectPair(
  contours: readonly Contour[],
  selection: Selection,
): { a: PointRef; b: PointRef } | null {
  const nodes: PointRef[] = []
  for (const target of selection.values()) {
    if (target.kind !== 'node') return null
    nodes.push({ contour: target.contour, point: target.point })
  }
  if (nodes.length !== 2) return null
  const [first, second] = nodes as [PointRef, PointRef]
  if (first.contour !== second.contour) return null
  const contour = contours[first.contour]
  if (!contour) return null
  const order = onCurve(contour)
  if (order.length < 4) return null
  const one = order.indexOf(first.point)
  const two = order.indexOf(second.point)
  if (one < 0 || two < 0) return null
  const follows = (from: number, to: number) =>
    to === (contour.closed ? (from + 1) % order.length : from + 1)
  if (follows(one, two)) return { a: first, b: second }
  if (follows(two, one)) return { a: second, b: first }
  return null
}

/** `AlignComponents`: place every auto-aligned component by its anchors. */
export function alignComponentsCommand(target: LayerTarget): Command {
  return { AlignComponents: { glyph: target.glyph, layer: target.layer } }
}
