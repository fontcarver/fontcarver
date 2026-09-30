/**
 * Routing pointer and key events to the active drawing tool.
 *
 * Each tool is its own state machine (`tools/<tool>.ts`); this is the strip of
 * wiring between them and the controller — it builds the tool for a `ToolId`,
 * hands events on and turns what comes back into the editor's own vocabulary:
 * a command to apply, a preview to publish, a dialog to open. Keeping it apart
 * from `editorController.ts` means adding a tool touches one factory and one
 * file, never the pointer grammar of the whole editor.
 */

import type { Anchor, Command, ComponentView, Contour } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'

import type { EditorPreview, PointerInput } from './editorTypes'
import type { Modifiers } from './gestures/drag'
import type { Hit } from './hitTest'
import type { EditOutline } from './outlineGeometry'
import type { LayerCurves } from './pathGeometry'
import type { LayerTarget } from './pointCommands'
import { AnnotationTool } from './tools/annotation'
import { LassoTool } from './tools/lasso'
import { EraserTool } from './tools/eraser'
import { KnifeTool } from './tools/knife'
import { MeasureTool } from './tools/measure'
import { PencilTool } from './tools/pencil'
import { PenTool } from './tools/pen'
import { RotateTool } from './tools/rotate'
import { ScaleTool } from './tools/scale'
import { ShapeTool } from './tools/shapes'
import {
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ShapeDialogRequest,
  type SnappedPoint,
  type ToolContext,
  type ToolKey,
  type ToolLabels,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './tools/toolTypes'
import type { ToolId } from './tools/registry'
import type { EditTarget, Selection } from './selection'

/** The tool for an id, or `null` when the controller handles it itself. */
export function createTool(id: ToolId): EditorTool | null {
  switch (id) {
    case 'annotation':
      return new AnnotationTool()
    case 'lasso':
      return new LassoTool()
    case 'eraser':
      return new EraserTool()
    case 'pen':
      return new PenTool()
    case 'pencil':
      return new PencilTool()
    case 'knife':
      return new KnifeTool()
    case 'rectangle':
      return new ShapeTool('rectangle')
    case 'ellipse':
      return new ShapeTool('ellipse')
    case 'measure':
      return new MeasureTool()
    case 'rotate':
      return new RotateTool()
    case 'scale':
      return new ScaleTool()
    // Select, Hand and Zoom are the controller's own gestures; Text stays
    // inert until the Text tab arrives.
    default:
      return null
  }
}

/** Where the tool context's inputs come from, read fresh for every event. */
export interface ToolHostSources {
  viewport: Viewport
  target: () => LayerTarget | null
  contours: () => readonly Contour[]
  outline: () => EditOutline
  anchors: () => readonly Anchor[]
  components: () => readonly ComponentView[]
  selection: () => Selection
  canSelect?: (target: EditTarget) => boolean
  curves: () => LayerCurves
  italicAngle?: () => number
  labels: () => ToolLabels
  snap: (point: Point2, suspend: boolean) => SnappedPoint
  hitTest: (screen: Point2) => Hit | null
  segmentShortcut: (
    hit: Hit,
    modifiers: Modifiers,
  ) => { command: Command; label: string } | null
  after: () => Promise<unknown> | undefined
  onError: (error: unknown) => void
}

/** What the host reports back to the controller. */
export interface ToolHostCallbacks {
  /** Apply one command as its own undo entry. */
  apply: (command: Command, label: string) => void
  setSelection: (selection: Selection) => void
  setPreview: (preview: EditorPreview | null) => void
  /** A tool's kernel gesture opened or closed. */
  setEditing: (editing: boolean) => void
  /** A gesture the next one must wait for. */
  setClosing: (closing: Promise<unknown>) => void
  openDialog: (request: ShapeDialogRequest) => void
  /** A tool gesture finished, so its transient feedback can be cleared. */
  gestureEnded: () => void
  repaint: () => void
}

export class ToolHost {
  private readonly sources: ToolHostSources
  private readonly callbacks: ToolHostCallbacks
  private tool: EditorTool | null = null

  constructor(sources: ToolHostSources, callbacks: ToolHostCallbacks) {
    this.sources = sources
    this.callbacks = callbacks
  }

  /** Everything the active tool is allowed to see, as of right now. */
  private context(): ToolContext {
    const sources = this.sources
    const target = sources.target()
    return {
      target,
      contours: sources.contours(),
      outline: sources.outline(),
      anchors: sources.anchors(),
      components: sources.components(),
      viewport: sources.viewport,
      selection: sources.selection(),
      canSelect: sources.canSelect,
      curves: sources.curves(),
      italicAngle: sources.italicAngle?.() ?? 0,
      labels: sources.labels(),
      snap: (point, suspend) => sources.snap(point, suspend === true),
      hitTest: sources.hitTest,
      segmentShortcut: sources.segmentShortcut,
      after: sources.after(),
      onError: sources.onError,
    }
  }

  /** A canvas pointer event in the two spaces a tool works in. */
  private pointer(input: PointerInput): ToolPointer {
    const screen = { x: input.x, y: input.y }
    return {
      screen,
      font: this.sources.viewport.toFont(screen),
      button: input.button,
      modifiers: input.modifiers,
    }
  }

  /** Whether a tool owns the pointer grammar right now. */
  get active(): boolean {
    return this.tool !== null
  }

  /** Whether the active tool has a gesture in progress. */
  get busy(): boolean {
    return this.tool?.busy === true
  }

  /** Switches tools, committing whatever the old one had open. */
  setTool(id: ToolId): void {
    if (this.tool?.id === id) return
    this.finish()
    this.tool = createTool(id)
    // The outgoing tool may have returned no outcome while still leaving an
    // idle overlay (for example Measure), so switching tools is itself a
    // reason to repaint.
    this.callbacks.repaint()
  }

  pointerDown(input: PointerInput): boolean {
    const pointer = this.pointer(input)
    return this.route((tool, context) => tool.pointerDown(pointer, context))
  }

  pointerMove(input: PointerInput): boolean {
    const pointer = this.pointer(input)
    return this.route((tool, context) => tool.pointerMove(pointer, context))
  }

  pointerUp(input: PointerInput): boolean {
    const pointer = this.pointer(input)
    return this.route((tool, context) => tool.pointerUp(pointer, context))
  }

  pointerLeave(): boolean {
    return this.route((tool, context) => tool.pointerLeave?.(context) ?? {})
  }

  keyDown(key: ToolKey): boolean {
    return this.route((tool, context) => tool.keyDown?.(key, context) ?? {})
  }

  setSpaceHeld(held: boolean): boolean {
    return this.route(
      (tool, context) => tool.setSpaceHeld?.(held, context) ?? {},
    )
  }

  /** Whether the active tool would consume this key. */
  claimsKey(key: string): boolean {
    const tool = this.tool
    if (!tool) return false
    return tool.claimsKey?.(key) ?? tool.busy
  }

  /** Transient geometry the active tool wants drawn. */
  overlay(): ToolOverlay {
    if (!this.tool) return NO_TOOL_OVERLAY
    return this.tool.overlay(this.context())
  }

  /** The cursor the active tool wants, or `null` when it owns none. */
  cursor(
    pointer: { screen: Point2; modifiers: Modifiers } | null,
  ): string | null {
    if (!this.tool) return null
    return this.tool.cursor(pointer, this.context())
  }

  /** Commits whatever is open; called when the editor goes away. */
  dispose(): void {
    this.finish()
    this.tool = null
  }

  /** Finishes the active tool while its current target is still installed. */
  finish(): void {
    const tool = this.tool
    if (!tool) return
    this.consume(tool.finish(this.context()))
  }

  private route(
    run: (tool: EditorTool, context: ToolContext) => ToolOutcome,
  ): boolean {
    const tool = this.tool
    if (!tool) return false
    const wasBusy = tool.busy
    const outcome = run(tool, this.context())
    this.consume(outcome)
    // A gesture that has just finished leaves transient feedback behind — the
    // snap halo it latched onto — which nothing else would clear.
    if (wasBusy && !tool.busy) {
      this.callbacks.gestureEnded()
      // `consume` paints the outcome before the controller clears feedback;
      // paint once more so a snap halo cannot survive in the pixels.
      this.callbacks.repaint()
    }
    return outcome.handled === true
  }

  private consume(outcome: ToolOutcome): void {
    if (outcome.selection) this.callbacks.setSelection(outcome.selection)
    if (outcome.preview !== undefined)
      this.callbacks.setPreview(outcome.preview)
    if (outcome.editing !== undefined)
      this.callbacks.setEditing(outcome.editing)
    if (outcome.closing) this.callbacks.setClosing(outcome.closing)
    if (outcome.dialog) this.callbacks.openDialog(outcome.dialog)
    if (outcome.apply)
      this.callbacks.apply(outcome.apply.command, outcome.apply.label)
    // An event no tool claimed changed nothing: repainting for every key press
    // the canvas merely *offered* would put the whole editor on the key path.
    if (
      outcome.handled === true ||
      outcome.apply !== undefined ||
      outcome.selection !== undefined ||
      outcome.preview !== undefined ||
      outcome.dialog !== undefined
    )
      this.callbacks.repaint()
  }
}
