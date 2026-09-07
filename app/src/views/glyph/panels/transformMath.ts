/**
 * The arithmetic behind the Transform panel (design spec §7.7).
 *
 * Everything the panel does is one affine composed in the browser and sent as
 * a single `TransformSelection`, so the maths is pure and testable: the panel
 * only collects numbers and names the undo entry. Coefficients are kurbo's
 * `[a, b, c, d, e, f]`, mapping `(x, y)` to `(a·x + c·y + e, b·x + d·y + f)`.
 */

import type { PointRef } from '../../../api'
import type { FontBox } from '../../../canvas/Viewport'

/** A kurbo affine as the commands carry it. */
export type Affine = [number, number, number, number, number, number]

/** The identity transform. */
export const IDENTITY: Affine = [1, 0, 0, 1, 0, 0]

/** `a ∘ b`: the transform that applies `b` first and then `a`. */
export function multiply(a: Affine, b: Affine): Affine {
  return [
    a[0] * b[0] + a[2] * b[1],
    a[1] * b[0] + a[3] * b[1],
    a[0] * b[2] + a[2] * b[3],
    a[1] * b[2] + a[3] * b[3],
    a[0] * b[4] + a[2] * b[5] + a[4],
    a[1] * b[4] + a[3] * b[5] + a[5],
  ]
}

/** A translation. */
export function translate(dx: number, dy: number): Affine {
  return [1, 0, 0, 1, dx, dy]
}

/** `transform` applied about `origin` rather than about (0, 0). */
export function about(
  origin: { x: number; y: number },
  transform: Affine,
): Affine {
  return multiply(
    multiply(translate(origin.x, origin.y), transform),
    translate(-origin.x, -origin.y),
  )
}

/** The nine origin handles, row-major from the top left (design spec §6.1). */
export const ORIGIN_HANDLES: readonly (readonly [number, number])[] = [
  [0, 1],
  [0.5, 1],
  [1, 1],
  [0, 0.5],
  [0.5, 0.5],
  [1, 0.5],
  [0, 0],
  [0.5, 0],
  [1, 0],
]

/** The font-space point one of the nine handles names inside a box. */
export function originPoint(
  box: FontBox,
  handle: number,
): { x: number; y: number } {
  const [fx = 0.5, fy = 0.5] = ORIGIN_HANDLES[handle] ?? [0.5, 0.5]
  return {
    x: box.x0 + (box.x1 - box.x0) * fx,
    y: box.y0 + (box.y1 - box.y0) * fy,
  }
}

/** Everything the Transform panel can ask for in one Apply. */
export interface TransformInput {
  /** Translation in font units. */
  dx: number
  dy: number
  /** Scale in percent; 100 leaves the axis alone. */
  scaleX: number
  scaleY: number
  /** Counter-clockwise rotation in degrees. */
  rotate: number
  /** Shear in degrees: `skewX` leans the vertical, `skewY` the horizontal. */
  skewX: number
  skewY: number
  flipH: boolean
  flipV: boolean
}

/** An Apply that changes nothing. */
export const NEUTRAL_TRANSFORM: TransformInput = {
  dx: 0,
  dy: 0,
  scaleX: 100,
  scaleY: 100,
  rotate: 0,
  skewX: 0,
  skewY: 0,
  flipH: false,
  flipV: false,
}

const RADIANS = Math.PI / 180

/**
 * The affine of one Apply.
 *
 * Order matters and is the one every editor uses: the shape is flipped, then
 * scaled, then sheared, then rotated — all about the chosen origin — and the
 * result is translated. Composing in any other order would make "rotate 30°
 * and move right" mean two different things depending on which field the user
 * filled in first.
 */
export function transformAffine(
  input: TransformInput,
  origin: { x: number; y: number },
): Affine {
  const sx = (input.scaleX / 100) * (input.flipH ? -1 : 1)
  const sy = (input.scaleY / 100) * (input.flipV ? -1 : 1)
  const scale: Affine = [sx, 0, 0, sy, 0, 0]
  const shear: Affine = [
    1,
    Math.tan(input.skewY * RADIANS),
    Math.tan(input.skewX * RADIANS),
    1,
    0,
    0,
  ]
  const angle = input.rotate * RADIANS
  const rotation: Affine = [
    Math.cos(angle),
    Math.sin(angle),
    -Math.sin(angle),
    Math.cos(angle),
    0,
    0,
  ]
  const linear = multiply(rotation, multiply(shear, scale))
  return multiply(translate(input.dx, input.dy), about(origin, linear))
}

/** Whether an Apply would leave the selection exactly where it is. */
export function isNeutral(input: TransformInput): boolean {
  return (
    input.dx === 0 &&
    input.dy === 0 &&
    input.scaleX === 100 &&
    input.scaleY === 100 &&
    input.rotate === 0 &&
    input.skewX === 0 &&
    input.skewY === 0 &&
    !input.flipH &&
    !input.flipV
  )
}

/** One thing align and distribute can move as a unit. */
export interface AlignItem {
  /** Bounding box of the item in font units. */
  box: FontBox
  /** Stored points the item is made of, empty for anchors and components. */
  points: readonly PointRef[]
  /** Anchor index, when the item is an anchor. */
  anchor?: number
  /** Component index, when the item is a component. */
  component?: number
}

/** The six alignments and the two distributions of design spec §7.7. */
export type AlignMode =
  | 'left'
  | 'centerH'
  | 'right'
  | 'top'
  | 'middle'
  | 'bottom'
  | 'distributeH'
  | 'distributeV'

/** How far one item has to move. */
export interface AlignMove {
  item: AlignItem
  dx: number
  dy: number
}

function spanOf(items: readonly AlignItem[]): FontBox {
  return {
    x0: Math.min(...items.map((item) => item.box.x0)),
    y0: Math.min(...items.map((item) => item.box.y0)),
    x1: Math.max(...items.map((item) => item.box.x1)),
    y1: Math.max(...items.map((item) => item.box.y1)),
  }
}

/**
 * Even spacing along one axis: the two outermost items stay where they are and
 * the gaps between the rest are equalised, which is what "distribute" means in
 * every drawing tool. Items are ordered by their own position first, so the
 * order they were selected in cannot change the result.
 */
function distribute(items: readonly AlignItem[], axis: 'x' | 'y'): AlignMove[] {
  if (items.length < 3) return []
  const low = axis === 'x' ? 'x0' : 'y0'
  const high = axis === 'x' ? 'x1' : 'y1'
  const sorted = [...items].sort((a, b) => a.box[low] - b.box[low])
  const first = sorted[0]
  const last = sorted[sorted.length - 1]
  if (!first || !last) return []
  const extent = last.box[high] - first.box[low]
  const occupied = sorted.reduce(
    (sum, item) => sum + (item.box[high] - item.box[low]),
    0,
  )
  const gap = (extent - occupied) / (sorted.length - 1)
  const moves: AlignMove[] = []
  let cursor = first.box[low]
  for (const item of sorted) {
    const delta = cursor - item.box[low]
    if (delta !== 0)
      moves.push(
        axis === 'x' ? { item, dx: delta, dy: 0 } : { item, dx: 0, dy: delta },
      )
    cursor += item.box[high] - item.box[low] + gap
  }
  return moves
}

/** How far each item has to move for one alignment or distribution. */
export function alignMoves(
  items: readonly AlignItem[],
  mode: AlignMode,
): AlignMove[] {
  if (items.length < 2) return []
  if (mode === 'distributeH') return distribute(items, 'x')
  if (mode === 'distributeV') return distribute(items, 'y')
  const span = spanOf(items)
  const moves: AlignMove[] = []
  for (const item of items) {
    let dx = 0
    let dy = 0
    switch (mode) {
      case 'left':
        dx = span.x0 - item.box.x0
        break
      case 'right':
        dx = span.x1 - item.box.x1
        break
      case 'centerH':
        dx = (span.x0 + span.x1) / 2 - (item.box.x0 + item.box.x1) / 2
        break
      case 'bottom':
        dy = span.y0 - item.box.y0
        break
      case 'top':
        dy = span.y1 - item.box.y1
        break
      case 'middle':
        dy = (span.y0 + span.y1) / 2 - (item.box.y0 + item.box.y1) / 2
        break
    }
    if (dx !== 0 || dy !== 0) moves.push({ item, dx, dy })
  }
  return moves
}
