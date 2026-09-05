/**
 * Assembling the frame the canvas paints.
 *
 * The controller decides what a gesture *means*; this module turns the state
 * that resulted into the transient layer the renderer draws — the marquee, the
 * snap halo, the alignment lines, the ghosts and the cursor HUD (design spec
 * §4.1 items 10 and 14, §6.2). Keeping it separate means the paint description
 * is a pure function of the gesture state and can be reasoned about — and
 * tested — without driving a pointer.
 */

import type { Anchor, Contour } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { EditingOverlay } from '../views/glyph/drawEditing'

import type { DragGesture } from './gestures/drag'
import { marqueeSize, type MarqueeRect } from './gestures/marquee'
import type { ComponentBounds, Hit } from './hitTest'
import { handlePolar, hudText, stemWidth, type HudLabels } from './hud'
import { nodeOfHandle, pointAt, type EditOutline } from './outlineGeometry'
import { selectedNodeRefs, type Selection } from './selection'
import type { SnapMatch } from './snapping'

/** How near the pointer must come to a component before its ghosts appear. */
export const COMPONENT_GHOST_REACH = 24

/** Everything the canvas needs from the controller to paint one frame. */
export interface EditorFrame {
  contours: readonly Contour[] | null
  anchors: readonly Anchor[] | null
  selected: ReadonlySet<string>
  overlay: EditingOverlay
  hud: string | null
  pointerScreen: Point2 | null
}

/** The gesture state a frame is built from. */
export interface FrameState {
  viewport: Viewport
  labels: HudLabels
  /** Geometry as drawn: the optimistic preview during a drag, else the layer. */
  contours: readonly Contour[]
  /** Preview geometry, or `null` when no gesture is previewing anything. */
  preview: { contours: readonly Contour[]; anchors: readonly Anchor[] } | null
  edit: EditOutline
  selection: Selection
  components: readonly ComponentBounds[]
  pointer: Point2 | null
  drag: DragGesture | null
  /** The rubber band on screen — a selection marquee or a zoom rectangle. */
  band: MarqueeRect | null
  /** Whether that band is a selection marquee rather than the zoom tool's. */
  bandIsMarquee: boolean
  snap: SnapMatch | null
  hover: Hit | null
}

const NOTHING_SELECTED: ReadonlySet<string> = new Set()

/** The ghost node shown where a click would insert a point on a segment. */
function insertGhost(state: FrameState): Point2 | null {
  if (state.drag || state.band) return null
  return state.hover?.target.kind === 'segment' ? state.hover.point : null
}

/** Faint corner nodes of any component the pointer is approaching. */
function componentGhosts(state: FrameState): readonly Point2[] {
  const { viewport, pointer } = state
  if (!pointer || state.drag) return []
  const font = viewport.toFont(pointer)
  const reach = COMPONENT_GHOST_REACH / (viewport.scale || 1)
  const ghosts: Point2[] = []
  for (const component of state.components) {
    const box = component.bbox
    if (!box) continue
    const [x0, y0, x1, y1] = box
    if (
      font.x < x0 - reach ||
      font.x > x1 + reach ||
      font.y < y0 - reach ||
      font.y > y1 + reach
    )
      continue
    ghosts.push(
      { x: x0, y: y0 },
      { x: x1, y: y0 },
      { x: x1, y: y1 },
      { x: x0, y: y1 },
    )
  }
  return ghosts
}

/** Distance between exactly two selected nodes, for the ⟂ readout. */
function stem(state: FrameState): number | null {
  const nodes = selectedNodeRefs(state.selection)
  if (nodes.length !== 2) return null
  const a = nodes[0] ? pointAt(state.contours, nodes[0]) : null
  const b = nodes[1] ? pointAt(state.contours, nodes[1]) : null
  return a && b ? stemWidth(a, b) : null
}

/** The cursor-HUD line for the gesture in progress, or `null` when idle. */
export function frameHud(state: FrameState): string | null {
  const { labels } = state
  if (state.band) {
    const box = state.band
    const scale = state.viewport.scale || 1
    const size = marqueeSize(box)
    return hudText(
      {
        kind: 'marquee',
        width: size.width / scale,
        height: size.height / scale,
      },
      labels,
    )
  }
  const drag = state.drag
  if (!drag) return null
  if (drag.plan.kind === 'handle') {
    const handle = drag.plan.refs[0]
    const point = handle ? pointAt(state.contours, handle) : null
    // `nodeOfHandle` walks the contour, so it also finds the node across a
    // wrap-around; comparing stored indexes would miss it and drop the HUD
    // back to Δx/Δy where §6.2 asks for ∠/ℓ.
    const nodeRef = handle ? nodeOfHandle(state.contours, handle) : null
    const node = nodeRef ? pointAt(state.contours, nodeRef) : null
    if (point && node) {
      const polar = handlePolar(node, point)
      return hudText(
        {
          kind: 'handle',
          delta: drag.currentDelta,
          angle: polar.angle,
          length: polar.length,
          snap: drag.snappedTo,
        },
        labels,
      )
    }
  }
  return hudText(
    {
      kind: 'move',
      delta: drag.currentDelta,
      stem: stem(state),
      snap: drag.snappedTo,
    },
    labels,
  )
}

/** Builds the whole frame description for one paint. */
export function buildFrame(state: FrameState): EditorFrame {
  const overlay: EditingOverlay = {
    marquee: state.band,
    snap: state.snap?.point ?? null,
    alignX: state.snap?.alignedX ?? null,
    alignY: state.snap?.alignedY ?? null,
    insertGhost: insertGhost(state),
    componentGhosts: componentGhosts(state),
  }
  return {
    contours: state.preview?.contours ?? null,
    anchors: state.preview?.anchors ?? null,
    selected:
      state.selection.size === 0
        ? NOTHING_SELECTED
        : new Set(state.selection.keys()),
    overlay,
    hud: frameHud(state),
    pointerScreen: state.pointer,
  }
}
