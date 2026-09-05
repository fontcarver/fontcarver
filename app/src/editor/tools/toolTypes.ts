/**
 * The contract every drawing and transform tool implements.
 *
 * A tool is a small state machine: pointer and key events plus a context go
 * in, and what comes out is a command to apply, geometry to preview and a
 * cursor to show. Nothing here touches the kernel, the DOM or React, so a
 * tool's whole grammar — "three clicks and a click on the first node close a
 * triangle" — is testable from plain objects, and the controller's only job is
 * to route events and carry the results.
 */

import type { Anchor, Command, ComponentView, Contour } from '../../api'
import type { Point2, Viewport } from '../../canvas/Viewport'

import type { EditorPreview } from '../editorTypes'
import type { Scheduler } from '../editSession'
import type { Modifiers } from '../gestures/drag'
import type { Hit } from '../hitTest'
import type { HudLabels } from '../hud'
import type { EditOutline } from '../outlineGeometry'
import type { LayerCurves } from '../pathGeometry'
import type { LayerTarget } from '../pointCommands'
import type { Selection } from '../selection'
import type { SnapMatch } from '../snapping'

import type { ToolId } from './registry'

/** A pointer event in both spaces, with the modifiers held at the time. */
export interface ToolPointer {
  /** Canvas-space position, in CSS pixels. */
  screen: Point2
  /** The same position in font units. */
  font: Point2
  button: number
  modifiers: Modifiers
}

/** A key event a tool may claim. */
export interface ToolKey {
  key: string
  modifiers: Modifiers
  repeat: boolean
}

/** Undo labels and readout words the tools need, already translated. */
export interface ToolLabels {
  drawPath: string
  extendPath: string
  draw: string
  cut: string
  rectangle: string
  ellipse: string
  rotate: string
  scale: string
  /** Prefix for the measure tool's distance readout. */
  distance: string
  /** Prefix for the scale tool's percentage readout. */
  percent: string
  hud: HudLabels
}

/** A snapped point together with what it latched onto. */
export interface SnappedPoint {
  point: Point2
  match: SnapMatch
}

/** Everything a tool needs to know about the editor it is running in. */
export interface ToolContext {
  /** The layer being edited, or `null` before one has loaded. */
  target: LayerTarget | null
  contours: readonly Contour[]
  outline: EditOutline
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
  viewport: Viewport
  selection: Selection
  /** Which curve flavour a new contour should be built in. */
  curves: LayerCurves
  labels: ToolLabels
  /** Snaps one font-space point; the identity when snapping is suspended. */
  snap: (point: Point2, suspend?: boolean) => SnappedPoint
  hitTest: (screen: Point2) => Hit | null
  /** The §4.3 modified click on a segment, shared with the Select tool. */
  segmentShortcut: (
    hit: Hit,
    modifiers: Modifiers,
  ) => { command: Command; label: string } | null
  /** A gesture the editor is still closing, which a new one waits for. */
  after: Promise<unknown> | undefined
  /** How a tool's applies are deferred; tests step frames by hand. */
  scheduler?: Scheduler
  /** Where a failed kernel call is reported. */
  onError: (error: unknown) => void
}

/** A request to open the shapes numeric dialog (design spec §9). */
export interface ShapeDialogRequest {
  kind: 'rectangle' | 'ellipse'
  /** Where the click landed, which seeds the dialog's x and y. */
  at: Point2
}

/** What a tool asks the editor to do after one event. */
export interface ToolOutcome {
  /** Apply as its own undo entry. */
  apply?: { command: Command; label: string }
  /** Replace the selection. */
  selection?: Selection
  /** Open the numeric shape dialog. */
  dialog?: ShapeDialogRequest
  /**
   * Optimistic geometry for the *existing* layer, as a transform gesture
   * produces; `null` clears it. New geometry a tool is still drawing belongs
   * in the overlay instead, because it is not part of the layer yet.
   */
  preview?: EditorPreview | null
  /** Whether the tool now holds a live kernel gesture. */
  editing?: boolean
  /** A close the next gesture has to wait for. */
  closing?: Promise<unknown>
  /** Whether the event was consumed; an unclaimed key falls through. */
  handled?: boolean
}

/** Nothing to do. */
export const NO_OUTCOME: ToolOutcome = {}

/** Transient geometry a tool draws on top of the layer. */
export interface ToolOverlay {
  /** Contours drawn as a live preview above the layer. */
  contours: readonly Contour[]
  /** Straight lines in font space: a cut, a measurement, a handle. */
  lines: readonly { from: Point2; to: Point2; dashed?: boolean }[]
  /** Points drawn as small markers: pen nodes, cut points, intersections. */
  marks: readonly Point2[]
  /** Short labels placed at a font-space point, e.g. a span length. */
  labels: readonly { at: Point2; text: string }[]
  /** The origin marker of a rotate or scale gesture. */
  origin: Point2 | null
  /** The cursor-HUD line this tool wants, or `null`. */
  hud: string | null
}

/** An overlay that draws nothing. */
export const NO_TOOL_OVERLAY: ToolOverlay = {
  contours: [],
  lines: [],
  marks: [],
  labels: [],
  origin: null,
  hud: null,
}

/** One tool's state machine. */
export interface EditorTool {
  readonly id: ToolId
  /** Whether a gesture of this tool is in progress. */
  readonly busy: boolean
  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome
  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome
  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome
  keyDown?(key: ToolKey, context: ToolContext): ToolOutcome
  /**
   * Whether this key would be consumed. The command registry asks before it
   * claims a shortcut, so Enter finishes a pen path rather than toggling the
   * smoothness of a node that happens to be selected.
   */
  claimsKey?(key: string): boolean
  keyUp?(key: ToolKey, context: ToolContext): ToolOutcome
  /** The pointer left the canvas. */
  pointerLeave?(context: ToolContext): ToolOutcome
  /** Space is held; the pen repositions its last node instead of panning. */
  setSpaceHeld?(held: boolean, context: ToolContext): ToolOutcome
  /** The tool is being put away: commit or discard whatever is open. */
  finish(context: ToolContext): ToolOutcome
  overlay(context: ToolContext): ToolOverlay
  /** The cursor for the current hover. */
  cursor(
    pointer: { screen: Point2; modifiers: Modifiers } | null,
    context: ToolContext,
  ): string
}

/** The distance a pointer must travel before a press counts as a drag. */
export const DRAG_THRESHOLD = 3

/** Whether a pointer has travelled far enough to be a drag, in CSS pixels. */
export function movedEnough(a: Point2, b: Point2, scale = 1): boolean {
  return Math.hypot(a.x - b.x, a.y - b.y) * scale >= DRAG_THRESHOLD
}

/** Constrains a vector to the nearest 45° direction, keeping its length. */
export function constrainVector(from: Point2, to: Point2): Point2 {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const step = Math.PI / 4
  const angle = Math.round(Math.atan2(dy, dx) / step) * step
  const length = Math.hypot(dx, dy)
  const x = Math.cos(angle) * length
  const y = Math.sin(angle) * length
  return {
    x: from.x + (Math.abs(x) < 1e-9 ? 0 : x),
    y: from.y + (Math.abs(y) < 1e-9 ? 0 : y),
  }
}
