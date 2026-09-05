/**
 * The Path menu's commands (design spec §11).
 *
 * `PathOp` is a whole-layer operation in the kernel — `fw-doc` hands the
 * layer's contours to `fw-geom` and takes the result back — so overlap
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
 * `JoinContours` requires `a` to be the *last* point of an open contour and
 * `b` an endpoint of a different one (it reverses `b`'s contour itself when
 * `b` is its last point), so the pair is ordered here rather than left to the
 * kernel to reject.
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
    if (end) ends.push({ ref, end })
  }
  if (ends.length !== 2) return null
  const [first, second] = ends
  if (!first || !second || first.ref.contour === second.ref.contour) return null
  if (first.end === 'last') return { a: first.ref, b: second.ref }
  if (second.end === 'last') return { a: second.ref, b: first.ref }
  return null
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
