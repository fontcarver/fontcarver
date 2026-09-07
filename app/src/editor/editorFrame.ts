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

import type { Anchor, ComponentView, Contour } from '../api'

import type { EditorPreview } from './editorTypes'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { EditingOverlay } from '../views/glyph/drawEditing'

import type { DragGesture } from './gestures/drag'
import { marqueeSize, type MarqueeRect } from './gestures/marquee'
import type { ComponentBounds, Hit } from './hitTest'
import { handlePolar, hudText, stemWidth, type HudLabels } from './hud'
import { nodeOfHandle, pointAt, type EditOutline } from './outlineGeometry'
import { selectedNodeRefs, type Selection } from './selection'
import type { SnapMatch } from './snapping'
import { NO_TOOL_OVERLAY, type ToolOverlay } from './tools/toolTypes'

/** How near the pointer must come to a component before its ghosts appear. */
export const COMPONENT_GHOST_REACH = 24

/** Everything the canvas needs from the controller to paint one frame. */
export interface EditorFrame {
  contours: readonly Contour[] | null
  anchors: readonly Anchor[] | null
  components: readonly ComponentView[] | null
  selected: ReadonlySet<string>
  overlay: EditingOverlay
  /** Transient geometry the active drawing tool wants drawn. */
  tool: ToolOverlay
  hud: string | null
  pointerScreen: Point2 | null
  /** Horizontal metrics a live sidebearing drag is showing, or `null`. */
  metrics: { width: number; lsb: number; rsb: number } | null
  /** Component the pointer is over, for the locked-component hatch. */
  hoveredComponent: number | null
}

/** The gesture state a frame is built from. */
export interface FrameState {
  viewport: Viewport
  labels: HudLabels
  /** Geometry as drawn: the optimistic preview during a drag, else the layer. */
  contours: readonly Contour[]
  /** Preview geometry, or `null` when no gesture is previewing anything. */
  preview: EditorPreview | null
  edit: EditOutline
  selection: Selection
  components: readonly ComponentBounds[]
  /** Ghost nodes of the layer's components, from the kernel. */
  componentPoints: readonly Point2[]
  pointer: Point2 | null
  drag: DragGesture | null
  /** The rubber band on screen — a selection marquee or a zoom rectangle. */
  band: MarqueeRect | null
  snap: SnapMatch | null
  hover: Hit | null
  /**
   * A gesture the drag/marquee pair does not cover — a sidebearing drag, a
   * guide pulled off a ruler — describing itself: the HUD line it wants, the
   * metrics it is previewing and the alignment lines it is drawing.
   */
  aside?: {
    hud?: string | null
    metrics?: { width: number; lsb: number; rsb: number } | null
    alignX?: number | null
    alignY?: number | null
  } | null
  /** What the active drawing tool wants drawn, if any. */
  tool?: ToolOverlay
}

const NOTHING_SELECTED: ReadonlySet<string> = new Set()

/** The ghost node shown where a click would insert a point on a segment. */
function insertGhost(state: FrameState): Point2 | null {
  if (state.drag || state.band) return null
  return state.hover?.target.kind === 'segment' ? state.hover.point : null
}

/**
 * Faint nodes of any component the pointer is approaching (§4.1 item 14).
 *
 * The kernel's flattened component points are the real thing to snap to and
 * therefore the real thing to show; a layer whose ghosts have not arrived yet
 * falls back to the corners of each component's bounds, which at least says
 * "there is a component here".
 */
function componentGhosts(state: FrameState): readonly Point2[] {
  const { viewport, pointer } = state
  if (!pointer || state.drag) return []
  const font = viewport.toFont(pointer)
  const reach = COMPONENT_GHOST_REACH / (viewport.scale || 1)
  const near = (x: number, y: number): boolean =>
    Math.abs(x - font.x) <= reach && Math.abs(y - font.y) <= reach
  const ghosts: Point2[] = []
  for (const point of state.componentPoints)
    if (near(point.x, point.y)) ghosts.push(point)
  if (ghosts.length > 0) return ghosts
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
  if (state.aside?.hud != null) return state.aside.hud
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
    alignX: state.aside?.alignX ?? state.snap?.alignedX ?? null,
    alignY: state.aside?.alignY ?? state.snap?.alignedY ?? null,
    insertGhost: insertGhost(state),
    componentGhosts: componentGhosts(state),
  }
  const tool = state.tool ?? NO_TOOL_OVERLAY
  return {
    contours: state.preview?.contours ?? null,
    anchors: state.preview?.anchors ?? null,
    components: state.preview?.components ?? null,
    tool,
    selected:
      state.selection.size === 0
        ? NOTHING_SELECTED
        : new Set(state.selection.keys()),
    overlay,
    // A tool's own readout wins: while it is drawing, the drag HUD is idle.
    hud: tool.hud ?? frameHud(state),
    pointerScreen: state.pointer,
    metrics: state.aside?.metrics ?? null,
    hoveredComponent:
      state.hover?.target.kind === 'component'
        ? state.hover.target.index
        : null,
  }
}
