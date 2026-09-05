/**
 * The glyph editor's input controller.
 *
 * React owns the canvas element and the data; this plain class owns what the
 * pointer and the keyboard *mean*. It keeps the selection, the live gesture
 * and the optimistic geometry, and hands the canvas a ready-made frame to
 * paint. Nothing here touches the DOM or React, so the whole interaction
 * grammar is testable with plain objects.
 *
 * The optimistic copy is the point of the design: a drag updates local
 * geometry synchronously on every pointer move and tells the kernel once per
 * animation frame, so the outline can never lag the pointer.
 */

import type {
  Anchor,
  Command,
  Contour,
  DocId,
  GlyphId,
  HandleMode,
  LayerOutline,
  PointRef,
} from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { SceneMetrics } from '../views/glyph/glyphScene'

import { buildFrame, type EditorFrame } from './editorFrame'
import type { HudLabels } from './hud'
import {
  hitTest,
  type Hit,
  type HitTestInput,
  type ScopedGuide,
} from './hitTest'
import {
  beginDrag,
  NO_MODIFIERS,
  type DragGesture,
  type Modifiers,
} from './gestures/drag'
import { marqueeRect, marqueeSelection, marqueeSize } from './gestures/marquee'
import { NudgeCoalescer, nudgeStep } from './gestures/nudge'
import {
  buildEditOutline,
  movedPointRefs,
  nearestSplitParameter,
  translatePoints,
  type EditOutline,
  type EditSegment,
} from './outlineGeometry'
import {
  commit,
  convertSegmentCommand,
  deleteCommand,
  insertPointCommand,
  nextSmoothFlag,
  smoothCommand,
  type LayerTarget,
} from './pointCommands'
import { snapDelta, type SnapMatch, type SnapNames } from './snapping'
import {
  EMPTY_SELECTION,
  onlyTarget,
  sameSelection,
  selectedNodeRefs,
  selectedPointRefs,
  selectionOf,
  unionSelection,
  type EditTarget,
  type Selection,
} from './selection'
import { cursorForHover, selectionForClick } from './tools/select'
import { HAND_CURSOR, HAND_DRAGGING_CURSOR } from './tools/hand'
import { ZOOM_MARQUEE_MIN, zoomAt, zoomToRect } from './tools/zoom'
import { toolById, type ToolId } from './tools/registry'

/** Undo labels and readout words, translated by the view. */
export interface EditorLabels {
  movePoints: (count: number) => string
  moveHandle: string
  reshapeSegment: string
  moveAnchor: string
  moveComponent: string
  nudgePoints: (count: number) => string
  toggleSmooth: string
  deletePoints: string
  insertPoint: string
  convertSegment: string
  /** Names the HUD gives a snap target, already translated. */
  snap: SnapNames
  hud: HudLabels
}

/** The glyph and layer being edited. */
export interface EditorSource {
  doc: DocId
  gid: GlyphId
  glyph: string
  outline: LayerOutline
  guides: readonly ScopedGuide[]
  metrics: SceneMetrics
  master: string | null
}

/** What the controller reports back to React. */
export interface EditorCallbacks {
  repaint: () => void
  onSelectionChange: (selection: Selection) => void
  onEditingChange: (editing: boolean) => void
  onZoomChange: (scale: number) => void
  /** A gesture finished: refetch the outline and reconcile. */
  onReload: () => void
  onCursorChange: (cursor: string) => void
  /**
   * A kernel call failed. Editing must not swallow transport errors: the shell
   * turns these into a toast, the same way a failed command does.
   */
  onError: (error: unknown) => void
}

/** A pointer event, normalised to canvas space. */
export interface PointerInput {
  x: number
  y: number
  button: number
  modifiers: Modifiers
}

/** Callbacks used until the view installs its own. */
const SILENT_CALLBACKS: EditorCallbacks = {
  repaint: () => undefined,
  onSelectionChange: () => undefined,
  onEditingChange: () => undefined,
  onZoomChange: () => undefined,
  onReload: () => undefined,
  onCursorChange: () => undefined,
  onError: () => undefined,
}

export class EditorController {
  private readonly viewport: Viewport
  private callbacks: EditorCallbacks = SILENT_CALLBACKS
  private labels: EditorLabels
  private source: EditorSource | null = null
  private edit: EditOutline = buildEditOutline([])

  private selection: Selection = EMPTY_SELECTION
  private drag: DragGesture | null = null
  private marquee: {
    origin: Point2
    current: Point2
    additive: boolean
  } | null = null
  private zoomBox: { origin: Point2; current: Point2 } | null = null
  private panning: { x: number; y: number } | null = null
  private hover: Hit | null = null
  private pointer: Point2 | null = null
  private snap: SnapMatch | null = null
  private preview: {
    contours: readonly Contour[]
    anchors: readonly Anchor[]
  } | null = null
  private nudge: NudgeCoalescer | null = null
  private nudgeRun: {
    base: readonly Contour[]
    total: Point2
    refs: PointRef[]
    anchors: readonly Anchor[]
  } | null = null
  private spaceHeld = false
  private tool: ToolId = 'select'
  private editingPublished = false

  constructor(viewport: Viewport, labels: EditorLabels) {
    this.viewport = viewport
    this.labels = labels
  }

  /**
   * Installs the callbacks. They arrive after construction because the view
   * that owns them is built around the controller, not the other way round.
   */
  setCallbacks(callbacks: EditorCallbacks): void {
    this.callbacks = callbacks
  }

  /**
   * Publishes whether a gesture is live.
   *
   * It is derived rather than toggled: a drag that finishes asynchronously
   * must not clear the flag that an arrow-key run set in the meantime, and the
   * data hooks rely on this flag to know when it is safe to reload.
   */
  private publishEditing(): void {
    const editing = this.drag !== null || this.nudge?.active === true
    if (editing === this.editingPublished) return
    this.editingPublished = editing
    this.callbacks.onEditingChange(editing)
  }

  /** Replaces the translated strings after a language change. */
  setLabels(labels: EditorLabels): void {
    this.labels = labels
  }

  /** Which tool the strip currently has active (or borrowed). */
  setTool(tool: ToolId): void {
    this.tool = tool
    this.updateCursor()
  }

  /**
   * Installs new data. Any live preview is dropped: the kernel has spoken.
   *
   * The selection survives a reload of the same glyph — an `edit_apply` makes
   * the view refetch, and losing the selection every frame of a drag would be
   * absurd — and is cleared only when a different glyph arrives.
   */
  setSource(source: EditorSource | null): void {
    const changedGlyph =
      source !== null &&
      (source.doc !== this.source?.doc || source.gid !== this.source?.gid)
    this.source = source
    this.edit = buildEditOutline(source?.outline.contours ?? [])
    this.preview = null
    if (!changedGlyph) return
    // Published unconditionally, not through `setSelection`: a freshly mounted
    // controller's selection is already empty, so the "nothing changed" guard
    // would leave the *store* holding the previous tab's selection — point
    // indexes that mean something else in this glyph.
    this.selection = EMPTY_SELECTION
    this.callbacks.onSelectionChange(EMPTY_SELECTION)
    this.callbacks.repaint()
  }

  /** Whether a drag gesture is currently live; read by dev verification. */
  get dragging(): boolean {
    return this.drag !== null
  }

  /** The selection, for the Info box and the status bar. */
  get currentSelection(): Selection {
    return this.selection
  }

  /** Replaces the selection and tells React. */
  setSelection(selection: Selection): void {
    if (sameSelection(this.selection, selection)) return
    this.selection = selection
    this.callbacks.onSelectionChange(selection)
    this.callbacks.repaint()
  }

  /** The frame the canvas paints. */
  frame(): EditorFrame {
    return buildFrame({
      viewport: this.viewport,
      labels: this.labels.hud,
      contours: this.contours(),
      preview: this.preview,
      edit: this.edit,
      selection: this.selection,
      components: this.source?.outline.components ?? [],
      pointer: this.pointer,
      drag: this.drag,
      marquee: this.marquee,
      zoomBox: this.zoomBox,
      snap: this.snap,
      hover: this.hover,
    })
  }

  /** Space is held: the canvas pans instead of selecting. */
  setSpaceHeld(held: boolean): void {
    this.spaceHeld = held
    this.updateCursor()
  }

  /** Pointer went down on the canvas. */
  pointerDown(input: PointerInput): void {
    void this.endNudge()
    // A gesture that never saw its pointer-up — a lost capture, a cancelled
    // pointer — must not survive into the next one.
    this.marquee = null
    this.zoomBox = null
    this.panning = null
    const at = { x: input.x, y: input.y }
    if (
      input.button === 1 ||
      this.spaceHeld ||
      this.effectiveTool() === 'hand'
    ) {
      this.panning = at
      this.callbacks.onCursorChange(HAND_DRAGGING_CURSOR)
      return
    }
    if (this.effectiveTool() === 'zoom') {
      this.zoomBox = { origin: at, current: at }
      return
    }
    if (this.effectiveTool() !== 'select') return

    const hit = this.hitAt(at)
    if (hit && this.handleSegmentShortcut(hit, input.modifiers)) return

    const next = selectionForClick(this.selection, hit, input.modifiers)
    this.setSelection(next)

    if (!hit) {
      this.marquee = {
        origin: at,
        current: at,
        additive: input.modifiers.shift || input.modifiers.mod,
      }
      return
    }
    this.startDrag(hit, this.viewport.toFont(at), input.modifiers)
  }

  /** Pointer moved over the canvas. */
  pointerMove(input: PointerInput): void {
    this.pointer = { x: input.x, y: input.y }
    if (this.panning) {
      this.viewport.pan(input.x - this.panning.x, input.y - this.panning.y)
      this.panning = { x: input.x, y: input.y }
      this.callbacks.repaint()
      return
    }
    if (this.zoomBox) {
      this.zoomBox = { ...this.zoomBox, current: { x: input.x, y: input.y } }
      this.callbacks.repaint()
      return
    }
    if (this.drag) {
      this.drag.move(this.viewport.toFont(this.pointer), input.modifiers)
      this.preview = this.drag.preview
      this.callbacks.repaint()
      return
    }
    if (this.marquee) {
      this.marquee = { ...this.marquee, current: { x: input.x, y: input.y } }
      this.callbacks.repaint()
      return
    }
    // Hovering repaints every move: the rulers carry a pointer marker and the
    // insert-node ghost follows the pointer along a segment.
    this.hover = this.hitAt(this.pointer)
    this.updateCursor(input.modifiers)
    this.callbacks.repaint()
  }

  /** Pointer released. */
  pointerUp(input: PointerInput): void {
    if (this.panning) {
      this.panning = null
      this.updateCursor(input.modifiers)
      return
    }
    if (this.zoomBox) {
      const rect = marqueeRect(this.zoomBox.origin, this.zoomBox.current)
      const size = marqueeSize(rect)
      if (size.width < ZOOM_MARQUEE_MIN && size.height < ZOOM_MARQUEE_MIN)
        zoomAt(this.viewport, this.zoomBox.origin, input.modifiers.alt)
      else zoomToRect(this.viewport, rect)
      this.zoomBox = null
      this.callbacks.onZoomChange(this.viewport.scale)
      this.callbacks.repaint()
      return
    }
    if (this.marquee) {
      const rect = marqueeRect(this.marquee.origin, this.marquee.current)
      const found = marqueeSelection(this.hitInput(), rect, {
        ignoreHandles: input.modifiers.alt,
      })
      this.setSelection(
        this.marquee.additive ? unionSelection(this.selection, found) : found,
      )
      this.marquee = null
      this.callbacks.repaint()
      return
    }
    void this.finishDrag()
  }

  /** Pointer left the canvas. */
  pointerLeave(): void {
    this.pointer = null
    this.hover = null
    this.callbacks.repaint()
  }

  /** Double-click toggles a node between smooth and corner. */
  doubleClick(input: PointerInput): void {
    const hit = this.hitAt({ x: input.x, y: input.y })
    if (hit?.target.kind !== 'node') return
    this.setSelection(selectionOf([hit.target]))
    void this.toggleSmooth(input.modifiers.alt)
  }

  /** Zooms at a canvas point, used by the wheel handler. */
  zoomAtPoint(at: Point2, factor: number): void {
    this.viewport.zoomAt(at, factor)
    this.callbacks.onZoomChange(this.viewport.scale)
  }

  /** Ends whatever gesture is running; called when the view unmounts. */
  dispose(): void {
    void this.drag?.cancel()
    void this.nudge?.end()
    this.drag = null
    this.nudge = null
  }

  /** Toggles smooth on the selected nodes; Alt keeps the outline unchanged. */
  async toggleSmooth(keepOutline: boolean): Promise<void> {
    const target = this.layerTarget()
    const nodes = selectedNodeRefs(this.selection)
    if (!target || nodes.length === 0) return
    const smooth = nextSmoothFlag(this.contours(), nodes)
    await this.runCommand(
      smoothCommand(target, nodes, smooth, !keepOutline),
      this.labels.toggleSmooth,
    )
  }

  /** Deletes the selected nodes; `break` opens the contour instead. */
  async deleteSelection(breakContour: boolean): Promise<void> {
    const target = this.layerTarget()
    const points = selectedPointRefs(this.selection)
    if (!target || points.length === 0) return
    await this.runCommand(
      deleteCommand(
        target,
        points,
        breakContour ? 'BreakContour' : 'KeepShape',
      ),
      this.labels.deletePoints,
    )
    this.setSelection(EMPTY_SELECTION)
  }

  /**
   * Applies one command as its own undo entry.
   *
   * A failure is reported and the view still reconciles: the kernel is the
   * source of truth, so after anything unexpected the honest move is to refetch
   * rather than keep drawing what we hoped had happened.
   */
  private async runCommand(command: Command, label: string): Promise<void> {
    const target = this.layerTarget()
    if (!target) return
    try {
      await commit(target, command, label)
    } catch (error) {
      this.callbacks.onError(error)
    } finally {
      this.callbacks.onReload()
    }
  }

  /** Selects every point of the layer. */
  selectAll(): void {
    const targets: EditTarget[] = this.edit.points.map((point) =>
      point.onCurve
        ? { kind: 'node', contour: point.ref.contour, point: point.ref.point }
        : {
            kind: 'handle',
            contour: point.ref.contour,
            point: point.ref.point,
          },
    )
    this.setSelection(selectionOf(targets))
  }

  /** Tab and Shift+Tab walk the on-curve points in stored order. */
  cycleNodes(direction: 1 | -1): void {
    const nodes = this.edit.points.filter((point) => point.onCurve)
    if (nodes.length === 0) return
    const current = onlyTarget(this.selection)
    const index =
      current && current.kind === 'node'
        ? nodes.findIndex(
            (node) =>
              node.ref.contour === current.contour &&
              node.ref.point === current.point,
          )
        : -1
    const next = nodes[(index + direction + nodes.length * 2) % nodes.length]
    if (!next) return
    this.setSelection(
      selectionOf([
        { kind: 'node', contour: next.ref.contour, point: next.ref.point },
      ]),
    )
  }

  /**
   * One arrow press.
   *
   * The move is shown immediately from a local copy — the kernel is told on
   * the next animation frame and the whole run becomes one undo entry — so an
   * arrow key feels the same as a drag rather than waiting on a round trip.
   */
  nudgeSelection(direction: Point2, modifiers: Modifiers): void {
    const target = this.layerTarget()
    const source = this.source
    const points = selectedPointRefs(this.selection)
    if (!target || !source || points.length === 0) return
    const step = nudgeStep(modifiers)
    const delta = { x: direction.x * step, y: direction.y * step }
    const handles: HandleMode = modifiers.alt ? 'PointsOnly' : 'WithHandles'
    this.nudge ??= new NudgeCoalescer({
      doc: target.doc,
      glyph: target.glyph,
      layer: target.layer,
      label: (count) => this.labels.nudgePoints(count),
      onEnd: () => this.closeNudge(),
      onError: (error) => this.callbacks.onError(error),
    })
    this.nudge.nudge(points, delta, handles)
    this.publishEditing()

    // Accumulate against the layer as the kernel last reported it: reloads are
    // deferred for the length of the run, so that geometry stays put.
    const base = this.nudgeRun?.base ?? source.outline.contours
    const total = this.nudgeRun
      ? {
          x: this.nudgeRun.total.x + delta.x,
          y: this.nudgeRun.total.y + delta.y,
        }
      : delta
    this.nudgeRun = {
      base,
      total,
      refs: movedPointRefs(base, points, handles),
      anchors: source.outline.anchors,
    }
    this.preview = {
      contours: translatePoints(base, this.nudgeRun.refs, total.x, total.y),
      anchors: this.nudgeRun.anchors,
    }
    this.callbacks.repaint()
  }

  /** Finalises a run of nudges: one undo entry, then a reload. */
  async endNudge(): Promise<void> {
    if (!this.nudge?.active) return
    try {
      await this.nudge.end()
    } catch (error) {
      this.callbacks.onError(error)
    }
  }

  /**
   * A nudge run closed, from the idle timer or from any other action. The
   * preview is handed back to the layer and reloads resume.
   */
  private closeNudge(): void {
    this.nudgeRun = null
    this.preview = null
    this.publishEditing()
    this.callbacks.onReload()
    this.callbacks.repaint()
  }

  /** Escape: cancel a live drag, or clear the selection. */
  escape(): void {
    if (this.drag) {
      const drag = this.drag
      this.drag = null
      this.preview = null
      void drag
        .cancel()
        .catch((error: unknown) => this.callbacks.onError(error))
        .finally(() => {
          this.publishEditing()
          this.callbacks.onReload()
        })
      this.callbacks.repaint()
      return
    }
    if (this.marquee) {
      this.marquee = null
      this.callbacks.repaint()
      return
    }
    this.setSelection(EMPTY_SELECTION)
  }

  private effectiveTool(): ToolId {
    return this.spaceHeld ? 'hand' : this.tool
  }

  private contours(): readonly Contour[] {
    return this.preview?.contours ?? this.source?.outline.contours ?? []
  }

  private anchors(): readonly Anchor[] {
    return this.preview?.anchors ?? this.source?.outline.anchors ?? []
  }

  private layerTarget(): LayerTarget | null {
    const source = this.source
    if (!source) return null
    return {
      doc: source.doc,
      glyph: source.glyph,
      layer: source.outline.layer,
    }
  }

  private hitInput(): HitTestInput {
    return {
      outline: this.edit,
      anchors: this.anchors(),
      guides: this.source?.guides ?? [],
      components: this.source?.outline.components ?? [],
      viewport: this.viewport,
      selection: this.selection,
    }
  }

  private hitAt(at: Point2): Hit | null {
    if (!this.source) return null
    // Alt does *not* hide handles from the hit test: over a handle it means
    // "keep the angle" and over a node "break the handle" (design spec §4.3).
    // Ignoring handles is a marquee behaviour, and the marquee applies it
    // itself.
    return hitTest(this.hitInput(), at)
  }

  private updateCursor(modifiers: Modifiers = NO_MODIFIERS): void {
    const tool = this.effectiveTool()
    if (tool === 'hand')
      return this.callbacks.onCursorChange(
        this.panning ? HAND_DRAGGING_CURSOR : HAND_CURSOR,
      )
    if (tool !== 'select')
      return this.callbacks.onCursorChange(toolById(tool).cursor)
    this.callbacks.onCursorChange(cursorForHover(this.hover, modifiers))
  }

  /**
   * Alt-click converts a line segment to a curve and Shift-click inserts a
   * node on it (design spec §4.3); both are single commands, so they must not
   * open a drag gesture.
   */
  private handleSegmentShortcut(hit: Hit, modifiers: Modifiers): boolean {
    const target = this.layerTarget()
    if (!target || hit.target.kind !== 'segment') return false
    const segment = this.segmentAt(hit.target.contour, hit.target.segment)
    // Alt only converts a *line*. On a curve it means "reshape keeping the
    // handle angles" (§4.3), and converting a curve to a curve would record an
    // empty undo entry and dirty the document for nothing.
    if (modifiers.alt && segment?.kind === 'line') {
      void this.runCommand(
        convertSegmentCommand(
          target,
          hit.target.contour,
          hit.target.segment,
          'Curve',
        ),
        this.labels.convertSegment,
      )
      return true
    }
    if (modifiers.shift) {
      // §4.3: the node lands on the nearest extremum or inflection, not
      // wherever the pointer happened to be.
      const clicked = Math.min(0.999, Math.max(0.001, hit.t ?? 0.5))
      const t = segment ? nearestSplitParameter(segment, clicked) : clicked
      void this.runCommand(
        insertPointCommand(
          target,
          hit.target.contour,
          hit.target.segment,
          Math.min(0.999, Math.max(0.001, t)),
        ),
        this.labels.insertPoint,
      )
      return true
    }
    return false
  }

  /** One geometric segment of the layer, by contour and index. */
  private segmentAt(contour: number, index: number): EditSegment | undefined {
    return this.edit.segments.find(
      (candidate) => candidate.contour === contour && candidate.index === index,
    )
  }

  private startDrag(hit: Hit, origin: Point2, modifiers: Modifiers): void {
    const source = this.source
    const target = this.layerTarget()
    if (!source || !target) return
    this.snap = null
    const drag = beginDrag(
      {
        doc: target.doc,
        glyph: target.glyph,
        layer: target.layer,
        contours: source.outline.contours,
        anchors: source.outline.anchors,
        selection: this.selection,
        labels: {
          movePoints: (count) => this.labels.movePoints(count),
          moveHandle: this.labels.moveHandle,
          reshapeSegment: this.labels.reshapeSegment,
          moveAnchor: this.labels.moveAnchor,
          moveComponent: this.labels.moveComponent,
        },
        onError: (error) => this.callbacks.onError(error),
        snap: (delta, moved) => {
          const match = snapDelta(this.snapInput(), delta, moved)
          this.snap = match
          return { delta: match.delta, label: match.label }
        },
      },
      hit,
      origin,
      modifiers,
    )
    if (!drag) return
    this.drag = drag
    this.publishEditing()
  }

  private snapInput() {
    const source = this.source
    const exclude = new Set(
      selectedPointRefs(this.selection).map(
        (ref) => `${ref.contour}:${ref.point}`,
      ),
    )
    for (const ref of this.drag?.plan.refs ?? [])
      exclude.add(`${ref.contour}:${ref.point}`)
    return {
      outline: this.edit,
      anchors: source?.outline.anchors ?? [],
      guides: source?.guides ?? [],
      metrics:
        source?.metrics ??
        ({
          upem: 1000,
          ascender: 800,
          descender: -200,
          xHeight: 500,
          capHeight: 700,
          italicAngle: 0,
          width: 0,
          lsb: null,
          rsb: null,
          verticalOrigin: null,
          verticalWidth: null,
        } satisfies SceneMetrics),
      names: this.labels.snap,
      viewport: this.viewport,
      exclude,
    }
  }

  private async finishDrag(): Promise<void> {
    const drag = this.drag
    if (!drag) return
    this.drag = null
    this.snap = null
    let changed = true
    try {
      changed = await drag.finish()
    } catch (error) {
      this.callbacks.onError(error)
    } finally {
      // However the gesture ended, the preview goes away, reloads resume and
      // the canvas repaints; leaving any of those behind strands the editor.
      this.preview = null
      this.publishEditing()
      // A gesture that failed part-way leaves the document in a state only the
      // kernel knows, so it is refetched even though nothing "changed".
      if (changed || drag.failed) this.callbacks.onReload()
      else this.callbacks.repaint()
    }
  }
}
