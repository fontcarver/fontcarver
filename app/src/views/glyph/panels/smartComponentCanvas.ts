/**
 * Smart-component handles on the main glyph canvas.
 *
 * A component whose base glyph exposes local axes gets one handle per axis,
 * drawn on a track beside its own bounds: dragging the handle sets that
 * instance's coordinate for that axis, and nothing else about the component
 * changes. Tracks run along the bottom, left, top and right edges in axis order,
 * so the first two axes — the ones a CJK radical usually exposes — read as a
 * horizontal and a vertical amount.
 *
 * The geometry and the value mapping are pure, so the canvas gesture and the
 * inspector's sliders cannot disagree about what a position means.
 */

import type { ComponentView, SmartAxis } from '../../../api'
import type { Point2, Viewport } from '../../../canvas/Viewport'
import type { Selection } from '../../../editor/selection'

import { smartValue } from './smartComponentValues'

/** Handles drawn for one component; further axes stay in the inspector. */
export const MAX_CANVAS_AXES = 4
/** Distance of a track from the component's bounds, in CSS pixels. */
const TRACK_OFFSET = 12
/** Hit radius of a handle, in CSS pixels. */
export const SMART_HANDLE_HIT = 7
/** Drawn half-size of a handle, in CSS pixels. */
const HANDLE_SIZE = 4
/** Positions a drag can reach along one track, matching the inspector's step. */
const STEPS = 100

/** One axis handle of one component. */
export interface SmartHandle {
  /** Index of the component in the layer. */
  component: number
  /** Index of the axis in the base glyph's local axes. */
  axis: number
  name: string
  min: number
  max: number
  /** Current value of this instance's coordinate. */
  value: number
  /** Track start in font units, where the axis is at its minimum. */
  from: Point2
  /** Track end in font units, where the axis is at its maximum. */
  to: Point2
  /** Handle position in font units. */
  at: Point2
}

function track(
  bbox: [number, number, number, number],
  axis: number,
  offset: number,
): { from: Point2; to: Point2 } {
  const [x0, y0, x1, y1] = bbox
  switch (axis % MAX_CANVAS_AXES) {
    case 0:
      return { from: { x: x0, y: y0 - offset }, to: { x: x1, y: y0 - offset } }
    case 1:
      return { from: { x: x0 - offset, y: y0 }, to: { x: x0 - offset, y: y1 } }
    case 2:
      return { from: { x: x0, y: y1 + offset }, to: { x: x1, y: y1 + offset } }
    default:
      return { from: { x: x1 + offset, y: y0 }, to: { x: x1 + offset, y: y1 } }
  }
}

function fraction(axis: { min: number; max: number }, value: number): number {
  const span = axis.max - axis.min
  if (!Number.isFinite(span) || Math.abs(span) < 1e-12) return 0
  return Math.max(0, Math.min(1, (value - axis.min) / span))
}

/** Every drawable handle of the selected smart components. */
export function smartHandles(
  components: readonly ComponentView[],
  selection: Selection,
  viewport: Viewport,
  glyphLocked = false,
): SmartHandle[] {
  if (glyphLocked || viewport.scale <= 0) return []
  const offset = TRACK_OFFSET / viewport.scale
  const handles: SmartHandle[] = []
  components.forEach((view, component) => {
    const axes: SmartAxis[] = view.smart_axes ?? []
    if (
      axes.length === 0 ||
      view.bbox === null ||
      view.component.locked ||
      !selection.has(`component:${component}`)
    )
      return
    const values = view.component.smart ?? {}
    axes.slice(0, MAX_CANVAS_AXES).forEach((axis, index) => {
      if (
        !Number.isFinite(axis.min) ||
        !Number.isFinite(axis.max) ||
        axis.max <= axis.min
      )
        return
      const { from, to } = track(
        view.bbox as [number, number, number, number],
        index,
        offset * (1 + Math.floor(index / MAX_CANVAS_AXES)),
      )
      const value = smartValue(axis, values)
      const share = fraction(axis, value)
      handles.push({
        component,
        axis: index,
        name: axis.name,
        min: axis.min,
        max: axis.max,
        value,
        from,
        to,
        at: {
          x: from.x + (to.x - from.x) * share,
          y: from.y + (to.y - from.y) * share,
        },
      })
    })
  })
  return handles
}

/** The handle under a canvas position, or `null`. */
export function smartHandleAt(
  handles: readonly SmartHandle[],
  viewport: Viewport,
  at: Point2,
): SmartHandle | null {
  let best: { handle: SmartHandle; distance: number } | null = null
  for (const handle of handles) {
    const screen = viewport.toScreen(handle.at)
    const distance = Math.hypot(screen.x - at.x, screen.y - at.y)
    if (distance <= SMART_HANDLE_HIT && (!best || distance < best.distance))
      best = { handle, distance }
  }
  return best?.handle ?? null
}

/**
 * The axis value a pointer position means, quantised to the inspector's step.
 *
 * The pointer is projected onto the handle's own track, so a track that runs
 * vertically only answers to vertical movement.
 */
export function smartValueFor(handle: SmartHandle, font: Point2): number {
  const span = {
    x: handle.to.x - handle.from.x,
    y: handle.to.y - handle.from.y,
  }
  const length = span.x * span.x + span.y * span.y
  if (!Number.isFinite(length) || length < 1e-12) return handle.value
  const along =
    ((font.x - handle.from.x) * span.x + (font.y - handle.from.y) * span.y) /
    length
  const clamped = Math.max(0, Math.min(1, along))
  const step = (handle.max - handle.min) / STEPS
  const value = handle.min + Math.round(clamped * STEPS) * step
  return Math.max(handle.min, Math.min(handle.max, value))
}

/** Draws the tracks, the handles and the dragged value. */
export function drawSmartHandles(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  handles: readonly SmartHandle[],
  colors: { track: string; handle: string; text: string; font: string },
  active: SmartHandle | null,
): void {
  if (handles.length === 0) return
  ctx.save()
  ctx.lineWidth = 1
  ctx.font = `10px ${colors.font}`
  ctx.textBaseline = 'middle'
  for (const handle of handles) {
    const from = viewport.toScreen(handle.from)
    const to = viewport.toScreen(handle.to)
    const at = viewport.toScreen(handle.at)
    ctx.strokeStyle = colors.track
    ctx.beginPath()
    ctx.moveTo(from.x, from.y)
    ctx.lineTo(to.x, to.y)
    ctx.stroke()
    const current =
      active &&
      active.component === handle.component &&
      active.axis === handle.axis
    ctx.fillStyle = current ? colors.handle : colors.track
    ctx.strokeStyle = colors.handle
    ctx.beginPath()
    ctx.rect(
      at.x - HANDLE_SIZE,
      at.y - HANDLE_SIZE,
      HANDLE_SIZE * 2,
      HANDLE_SIZE * 2,
    )
    ctx.fill()
    ctx.stroke()
    if (current) {
      ctx.fillStyle = colors.text
      ctx.textAlign = 'left'
      const label = `${handle.name} ${Math.round(handle.value * 100) / 100}`
      ctx.fillText(label, at.x + HANDLE_SIZE + 3, at.y)
    }
  }
  ctx.restore()
}
