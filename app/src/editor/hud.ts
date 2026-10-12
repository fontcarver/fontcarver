/**
 * The cursor HUD (design spec §6.2).
 *
 * A small inverted pill that follows the pointer *during drags only* and reads
 * out the number the gesture is about: the delta for a move, the polar form
 * for a handle, the size for a marquee, and the stem width when two nodes are
 * being pulled apart. Snapping adds the name of what was snapped to.
 *
 * It is painted on the canvas rather than rendered as DOM, so it cannot lag a
 * pointer that is already driving a 60 fps repaint.
 */

import type { Point2 } from '../canvas/Viewport'

import { formatUnits } from './fieldValue'

/** Labels the HUD needs, supplied translated by the view. */
export interface HudLabels {
  deltaX: string
  deltaY: string
  angle: string
  length: string
  width: string
  height: string
  stem: string
}

/** What the HUD is describing. */
export type HudState =
  | {
      kind: 'move'
      delta: Point2
      /** Distance between exactly two selected nodes, when there are two. */
      stem?: number | null
      snap?: string | null
    }
  | {
      kind: 'handle'
      delta: Point2
      angle: number
      length: number
      snap?: string | null
    }
  | { kind: 'marquee'; width: number; height: number }

function signed(value: number): string {
  const text = formatUnits(Math.abs(value))
  // A real minus sign, not a hyphen: the HUD is tabular and typographic.
  return value < 0 ? `−${text}` : text
}

/** The one-line HUD text, or `null` when nothing should be shown. */
export function hudText(
  state: HudState | null,
  labels: HudLabels,
): string | null {
  if (!state) return null
  if (state.kind === 'marquee')
    return `${labels.width} ${formatUnits(state.width)} × ${labels.height} ${formatUnits(state.height)}`
  if (state.kind === 'handle') {
    const parts = [
      `${labels.angle} ${state.angle.toFixed(1)}°`,
      `${labels.length} ${formatUnits(state.length)}`,
    ]
    if (state.snap) parts.push(state.snap)
    return parts.join('  ')
  }
  const parts = [
    `${labels.deltaX} ${signed(state.delta.x)}`,
    `${labels.deltaY} ${signed(state.delta.y)}`,
  ]
  if (state.stem !== null && state.stem !== undefined)
    parts.push(`${labels.stem} ${formatUnits(state.stem)}`)
  if (state.snap) parts.push(state.snap)
  return parts.join('  ')
}

/** Polar form of a handle relative to the node it belongs to. */
export function handlePolar(
  node: Point2,
  handle: Point2,
): { angle: number; length: number } {
  const dx = handle.x - node.x
  const dy = handle.y - node.y
  return {
    angle: (Math.atan2(dy, dx) * 180) / Math.PI,
    length: Math.hypot(dx, dy),
  }
}

/** Distance between two points, used for the stem-width readout. */
export function stemWidth(a: Point2, b: Point2): number {
  return Math.hypot(a.x - b.x, a.y - b.y)
}
