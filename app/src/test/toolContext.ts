import type { Contour } from '../api'
import type { Point2 } from '../canvas/Viewport'
import { Viewport } from '../canvas/Viewport'
import { immediateScheduler } from '../editor/editSession'
import { hitTest } from '../editor/hitTest'
import { buildEditOutline } from '../editor/outlineGeometry'
import { EMPTY_SELECTION } from '../editor/selection'
import type { LayerCurves } from '../editor/pathGeometry'
import type {
  ToolContext,
  ToolLabels,
  ToolPointer,
} from '../editor/tools/toolTypes'
import { NO_MODIFIERS, type Modifiers } from '../editor/gestures/drag'

/** Undo labels and readouts for the drawing tools, in plain English. */
export const TOOL_LABELS: ToolLabels = {
  drawPath: 'Draw path',
  extendPath: 'Extend path',
  draw: 'Draw',
  cut: 'Cut',
  rectangle: 'Rectangle',
  ellipse: 'Ellipse',
  rotate: 'Rotate',
  scale: 'Scale',
  distance: 'd',
  percent: '⤢',
  hud: {
    deltaX: 'Δx',
    deltaY: 'Δy',
    angle: '∠',
    length: 'ℓ',
    width: 'w',
    height: 'h',
    stem: '⟂',
  },
}

/** A viewport with one CSS pixel per font unit and the origin at (0, 300). */
export function toolViewport(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(400, 400, 1)
  viewport.scale = 1
  viewport.offset = { x: 0, y: 300 }
  return viewport
}

/** Options a test may vary about the tool context. */
export interface ToolContextOptions {
  contours?: readonly Contour[]
  curves?: LayerCurves
  viewport?: Viewport
  /** Snapped positions, by `${x}:${y}`; anything else stays put. */
  snapTo?: (point: Point2) => Point2 | null
}

/** A tool context over a plain layer, with the real hit test and no kernel. */
export function makeToolContext(options: ToolContextOptions = {}): ToolContext {
  const contours = options.contours ?? []
  const viewport = options.viewport ?? toolViewport()
  const outline = buildEditOutline(contours)
  return {
    target: { doc: 1, glyph: 'a', layer: 'layer-1' },
    contours,
    outline,
    anchors: [],
    components: [],
    viewport,
    selection: EMPTY_SELECTION,
    curves: options.curves ?? 'cubic',
    labels: TOOL_LABELS,
    snap: (point, suspend) => {
      const snapped = suspend ? null : (options.snapTo?.(point) ?? null)
      return {
        point: snapped ?? point,
        match: {
          delta: snapped
            ? { x: snapped.x - point.x, y: snapped.y - point.y }
            : { x: 0, y: 0 },
          label: snapped ? 'node' : null,
          alignedX: null,
          alignedY: null,
          point: snapped,
        },
      }
    },
    hitTest: (screen) =>
      hitTest(
        {
          outline,
          anchors: [],
          guides: [],
          components: [],
          viewport,
          selection: EMPTY_SELECTION,
        },
        screen,
      ),
    segmentShortcut: () => null,
    after: undefined,
    // Applies land the moment they are queued, so a test steps the protocol
    // itself rather than waiting on an animation frame.
    scheduler: immediateScheduler,
    onError: () => undefined,
  }
}

/** A pointer event at a font-space position, mapped through the viewport. */
export function toolPointer(
  context: ToolContext,
  font: Point2,
  modifiers: Modifiers = NO_MODIFIERS,
  button = 0,
): ToolPointer {
  return {
    screen: context.viewport.toScreen(font),
    font,
    button,
    modifiers,
  }
}

/** One click: pointer down and straight back up at the same place. */
export function click(
  tool: {
    pointerDown: (p: ToolPointer, c: ToolContext) => unknown
    pointerUp: (p: ToolPointer, c: ToolContext) => unknown
  },
  context: ToolContext,
  font: Point2,
  modifiers: Modifiers = NO_MODIFIERS,
): unknown {
  const pointer = toolPointer(context, font, modifiers)
  tool.pointerDown(pointer, context)
  return tool.pointerUp(pointer, context)
}
