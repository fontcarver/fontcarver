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

import type { Anchor, Command, ComponentView, Contour } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'

import {
  allTargets,
  cycledSelection,
  segmentShortcut,
  startDragGesture,
} from './editorActions'
import { buildFrame, type EditorFrame } from './editorFrame'
import {
  SILENT_CALLBACKS,
  type EditorCallbacks,
  type EditorLabels,
  type EditorPreview,
  type EditorSource,
  type PointerInput,
} from './editorTypes'
import { hitTest, type Hit, type HitTestInput } from './hitTest'
import { NO_MODIFIERS, type DragGesture, type Modifiers } from './gestures/drag'
import { marqueeSelection } from './gestures/marquee'
import { NUDGE_STEPS, nudgeStep } from './gestures/nudge'
import { NudgeRunner } from './nudgeRun'
import { buildEditOutline, type EditOutline } from './outlineGeometry'
import {
  commit,
  deleteCommand,
  nextSmoothFlag,
  smoothCommand,
  type LayerTarget,
} from './pointCommands'
import type { SnapSettings } from './snapping'
import { SnapRunner, type SnapSources } from './editorSnapping'
import { ToolHost } from './toolHost'
import type { ToolKey } from './tools/toolTypes'
import {
  EMPTY_SELECTION,
  sameSelection,
  selectionOf,
  selectedNodeRefs,
  selectedPointRefs,
  unionSelection,
  type Selection,
} from './selection'
import { cursorForHover, selectionForClick } from './tools/select'
import { HAND_CURSOR, HAND_DRAGGING_CURSOR } from './tools/hand'

import { toolById, type ToolId } from './tools/registry'
import { ViewGestures } from './viewGestures'

export type { ShapeDialogRequest, ToolOverlay } from './tools/toolTypes'
export type {
  EditorCallbacks,
  EditorLabels,
  EditorPreview,
  EditorSource,
  PointerInput,
} from './editorTypes'

export class EditorController {
  private readonly viewport: Viewport
  private callbacks: EditorCallbacks = SILENT_CALLBACKS
  private labels: EditorLabels
  private source: EditorSource | null = null
  private edit: EditOutline = buildEditOutline([])
  private editBase: readonly Contour[] = []
  private readonly editCache = new WeakMap<readonly Contour[], EditOutline>()

  private selection: Selection = EMPTY_SELECTION
  private drag: DragGesture | null = null
  private readonly view = new ViewGestures()
  private hover: Hit | null = null
  private pointer: Point2 | null = null
  private preview: EditorPreview | null = null
  private readonly nudge: NudgeRunner
  private readonly tools: ToolHost
  private spaceHeld = false
  private toolEditing = false
  private tool: ToolId = 'select'
  private readonly snapping = new SnapRunner()
  private nudgeSteps: readonly [number, number, number] = NUDGE_STEPS
  private editingPublished = false
  /** The gesture that is closing, if any; a new one begins after it. */
  private closing: Promise<void> | undefined

  constructor(viewport: Viewport, labels: EditorLabels) {
    this.viewport = viewport
    this.labels = labels
    this.nudge = new NudgeRunner({
      onEnd: () => this.closeNudge(),
      onError: (error) => this.callbacks.onError(error),
    })
    this.tools = new ToolHost(
      {
        viewport,
        target: () => this.layerTarget(),
        contours: () => this.contours(),
        outline: () => this.editOutline(),
        anchors: () => this.anchors(),
        components: () => this.components(),
        selection: () => this.selection,
        curves: () => this.source?.curves ?? 'cubic',
        labels: () => this.labels.tools,
        snap: (point, suspend) =>
          this.snapping.forPoint(this.snapSources(), point, suspend),
        hitTest: (screen) => this.hitAt(screen),
        segmentShortcut: (hit, modifiers) => {
          const target = this.layerTarget()
          return target
            ? segmentShortcut(
                target,
                this.labels,
                this.editOutline(),
                hit,
                modifiers,
              )
            : null
        },
        after: () => this.closing,
        onError: (error) => this.callbacks.onError(error),
      },
      {
        apply: (command, label) => void this.runCommand(command, label),
        setSelection: (selection) => this.setSelection(selection),
        setPreview: (preview) => this.setPreview(preview),
        setEditing: (editing: boolean) => {
          this.toolEditing = editing
          this.publishEditing()
          if (!editing) this.callbacks.onReload()
        },
        setClosing: (closing) => void this.gate(closing),
        openDialog: (request) => this.callbacks.onShapeDialog(request),
        repaint: () => this.callbacks.repaint(),
      },
    )
  }

  /** Where the snap targets come from, as of right now. */
  private snapSources(): SnapSources {
    return {
      outline: this.editOutline(),
      source: this.source,
      selection: this.selection,
      drag: this.drag,
      labels: this.labels,
      viewport: this.viewport,
    }
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
    const editing = this.drag !== null || this.nudge.active || this.toolEditing
    if (editing === this.editingPublished) return
    this.editingPublished = editing
    this.callbacks.onEditingChange(editing)
  }

  /** Records the optimistic geometry and tells the panels that read it. */
  private setPreview(preview: EditorPreview | null): void {
    this.preview = preview
    this.callbacks.onPreview(preview)
  }

  /** Replaces the translated strings after a language change. */
  setLabels(labels: EditorLabels): void {
    this.labels = labels
  }

  /**
   * Whether snapping is on. Ctrl suspends it for one gesture (§4.2); this is
   * the standing preference behind that, which the status bar reports.
   */
  setSnapping(enabled: boolean): void {
    this.snapping.setEnabled(enabled)
  }

  /**
   * Which snap families are live and how coarse the grid is, from the kernel's
   * preferences. Read once per tab mount; there is no prefs-changed event.
   */
  setSnapSettings(settings: SnapSettings): void {
    this.snapping.setSettings(settings)
  }

  /** Arrow-key distances from the preferences: plain, Shift, Ctrl. */
  setNudgeSteps(steps: readonly [number, number, number]): void {
    this.nudgeSteps = steps
  }

  /** The grid step the canvas should draw, or zero when there is no grid. */
  get gridSize(): number {
    return this.snapping.gridSize
  }

  /** Which tool the strip currently has active (or borrowed). */
  setTool(tool: ToolId): void {
    void this.settle()
    this.tool = tool
    this.tools.setTool(tool)
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
    this.nudge.reloaded()
    this.edit = buildEditOutline(source?.outline.contours ?? [])
    this.editBase = source?.outline.contours ?? []
    this.setPreview(null)
    if (!changedGlyph) return
    // Published unconditionally, not through `setSelection`: a freshly mounted
    // controller's selection is already empty, so the "nothing changed" guard
    // would leave the *store* holding the previous tab's selection — point
    // indexes that mean something else in this glyph.
    this.selection = EMPTY_SELECTION
    this.callbacks.onSelectionChange(EMPTY_SELECTION)
    this.callbacks.repaint()
  }

  /** The pan/zoom transform, exposed for tests and dev verification. */
  get viewportForTest(): Viewport {
    return this.viewport
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
      edit: this.editOutline(),
      selection: this.selection,
      components: this.components(),
      pointer: this.pointer,
      drag: this.drag,
      band: this.view.band,
      snap: this.snapping.current,
      hover: this.hover,
      tool: this.tools.overlay(),
    })
  }

  /** Space is held: the canvas pans instead of selecting. */
  setSpaceHeld(held: boolean): void {
    this.spaceHeld = held
    // Space pans, except inside a live pen press, where §5 gives it to the
    // tool so it can reposition the node it is placing.
    if (this.tools.busy) this.tools.setSpaceHeld(held)
    this.updateCursor()
  }

  /** Pointer went down on the canvas. */
  pointerDown(input: PointerInput): void {
    void this.settle()
    // A gesture that never saw its pointer-up — a lost capture, a cancelled
    // pointer — must not survive into the next one.
    this.view.reset()
    const at = { x: input.x, y: input.y }
    if (
      input.button === 1 ||
      this.spaceHeld ||
      this.effectiveTool() === 'hand'
    ) {
      this.view.beginPan(at)
      this.callbacks.onCursorChange(HAND_DRAGGING_CURSOR)
      return
    }
    if (this.effectiveTool() === 'zoom') {
      this.view.beginZoom(at)
      return
    }
    if (this.effectiveTool() !== 'select') {
      this.tools.pointerDown(input)
      return
    }

    const hit = this.hitAt(at)
    if (hit && this.handleSegmentShortcut(hit, input.modifiers)) return

    this.setSelection(selectionForClick(this.selection, hit, input.modifiers))
    if (!hit) {
      this.view.beginMarquee(at, {
        additive: input.modifiers.shift || input.modifiers.mod,
        // Both halves of one gesture read the same moment: Alt decided when
        // the band was started, not whatever is held when it is released.
        ignoreHandles: input.modifiers.alt,
      })
      return
    }
    this.startDrag(hit, this.viewport.toFont(at), input.modifiers)
  }

  /** Pointer moved over the canvas. */
  pointerMove(input: PointerInput): void {
    this.pointer = { x: input.x, y: input.y }
    if (this.view.move(this.viewport, this.pointer)) {
      this.callbacks.repaint()
      return
    }
    if (this.drag) {
      this.drag.move(this.viewport.toFont(this.pointer), input.modifiers)
      this.setPreview(this.drag.preview)
      this.callbacks.repaint()
      return
    }
    if (this.toolsOwnPointer()) {
      this.tools.pointerMove(input)
      this.updateCursor(input.modifiers)
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
    if (this.view.endPan()) {
      this.updateCursor(input.modifiers)
      return
    }
    if (this.view.endZoom(this.viewport, input.modifiers.alt)) {
      this.callbacks.onZoomChange(this.viewport.scale)
      this.callbacks.repaint()
      return
    }
    const marquee = this.view.endMarquee()
    if (marquee) {
      const found = marqueeSelection(this.hitInput(), marquee.rect, {
        ignoreHandles: marquee.ignoreHandles,
      })
      this.setSelection(
        marquee.additive ? unionSelection(this.selection, found) : found,
      )
      this.callbacks.repaint()
      return
    }
    if (this.toolsOwnPointer()) {
      this.tools.pointerUp(input)
      return
    }
    void this.settle()
  }

  /** Pointer left the canvas. */
  pointerLeave(): void {
    this.pointer = null
    this.hover = null
    if (this.tools.active) this.tools.pointerLeave()
    this.callbacks.repaint()
  }

  /**
   * Offers a key to the active tool. Enter finishes a pen path and Escape
   * abandons one, so both have to reach the tool before the command registry
   * and the editor's own Escape do anything with them.
   */
  toolKey(key: ToolKey): boolean {
    return this.tools.keyDown(key)
  }

  /** Whether the active tool would consume this key; read by the commands. */
  claimsKey(key: string): boolean {
    return this.tools.claimsKey(key)
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
    void this.nudge.end()
    this.tools.dispose()
    this.drag = null
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
    // `doc.apply` finalises the active edit, so an open run has to be closed
    // before it rather than by it.
    await this.settle()
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
    this.setSelection(allTargets(this.editOutline()))
  }

  /** Tab and Shift+Tab walk the on-curve points in stored order. */
  cycleNodes(direction: 1 | -1): void {
    const next = cycledSelection(this.editOutline(), this.selection, direction)
    if (next) this.setSelection(next)
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
    const step = nudgeStep(modifiers, this.nudgeSteps)
    this.setPreview(
      this.nudge.press({
        target,
        contours: this.contours(),
        anchors: this.anchors(),
        components: this.components(),
        refs: points,
        delta: { x: direction.x * step, y: direction.y * step },
        handles: modifiers.alt ? 'PointsOnly' : 'WithHandles',
        label: (count) => this.labels.nudgePoints(count),
        // A press that lands one round trip after a settle-, pointer-up- or
        // tool-switch-driven close must not overtake that close's last apply.
        after: this.closing,
      }),
    )
    this.publishEditing()
    this.callbacks.repaint()
  }

  /** Finalises a run of nudges: one undo entry, then a reload. */
  endNudge(): Promise<void> {
    return this.settle()
  }

  /**
   * Records the gesture that is closing, so the next one begins behind it.
   *
   * Only the gesture that is *still* closing should gate the next one: a gate
   * that is never dropped would make one unanswered call block every gesture
   * after it, not just its successor.
   */
  private gate(closing: Promise<unknown>): Promise<void> {
    const tracked = closing
      .then(
        () => undefined,
        () => undefined,
      )
      .finally(() => {
        if (this.closing === tracked) this.closing = undefined
      })
    this.closing = tracked
    return tracked
  }

  /**
   * Closes whatever gesture is still open and reports when it is done.
   *
   * `doc.edit_begin` and `doc.apply` both finalise the active edit in the
   * kernel, so a gesture that starts while an earlier one is still flushing
   * would invalidate that gesture's last apply — the arrow press or the last
   * frame of the drag would be dropped and an error raised. Every new gesture,
   * whichever tool it belongs to, waits for this first; a drag counts, because
   * a pointer-down that never saw its pointer-up leaves one open.
   */
  settle(): Promise<void> {
    if (this.drag) return this.gate(this.finishDrag())
    if (this.nudge.active) return this.gate(this.nudge.end())
    return this.closing ?? Promise.resolve()
  }

  /**
   * A nudge run closed, from the idle timer or from any other action. The
   * preview is handed back to the layer and reloads resume.
   */
  private closeNudge(): void {
    // A drag that started while the run was closing owns the preview now:
    // clearing it here would blank the dragged geometry for one frame and
    // request a reload the drag has deliberately deferred.
    if (this.drag) {
      this.publishEditing()
      return
    }
    this.setPreview(null)
    this.publishEditing()
    this.callbacks.onReload()
    this.callbacks.repaint()
  }

  /** Escape: cancel a live drag, or clear the selection. */
  escape(): void {
    if (
      this.tools.keyDown({
        key: 'Escape',
        modifiers: NO_MODIFIERS,
        repeat: false,
      })
    )
      return
    if (this.drag) {
      const drag = this.drag
      this.drag = null
      this.setPreview(null)
      void this.gate(
        drag
          .cancel()
          .catch((error: unknown) => this.callbacks.onError(error))
          .finally(() => {
            this.publishEditing()
            this.callbacks.onReload()
          }),
      )
      this.callbacks.repaint()
      return
    }
    if (this.view.active) {
      this.view.reset()
      this.callbacks.repaint()
      return
    }
    this.setSelection(EMPTY_SELECTION)
  }

  private effectiveTool(): ToolId {
    return this.spaceHeld ? 'hand' : this.tool
  }

  /**
   * The editable index of the geometry currently on screen.
   *
   * It follows the optimistic preview, not just the last fetched layer: while a
   * nudge run is open reloads are deferred, so a hit test or a drag based on the
   * fetched outline would aim at where the points *used* to be. Rebuilt only
   * when the geometry it indexes actually changes.
   */
  private editOutline(): EditOutline {
    const contours = this.contours()
    if (contours === this.editBase) return this.edit
    // Two callers index the same geometry within one pointer move (the snap
    // callback before the preview is replaced, the frame after it), and a
    // gesture allocates a fresh contour array every move, so the identity
    // check alone rebuilds the whole index twice per move. The cache is keyed
    // on the array itself and is weak, so it costs nothing once a preview is
    // dropped.
    const cached = this.editCache.get(contours)
    const outline = cached ?? buildEditOutline(contours)
    if (!cached) this.editCache.set(contours, outline)
    this.editBase = contours
    this.edit = outline
    return outline
  }

  private contours(): readonly Contour[] {
    return this.preview?.contours ?? this.source?.outline.contours ?? []
  }

  private components(): readonly ComponentView[] {
    return this.preview?.components ?? this.source?.outline.components ?? []
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
      outline: this.editOutline(),
      anchors: this.anchors(),
      guides: this.source?.guides ?? [],
      components: this.components(),
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
        this.view.panningNow ? HAND_DRAGGING_CURSOR : HAND_CURSOR,
      )
    const custom = this.tools.active
      ? this.tools.cursor(
          this.pointer ? { screen: this.pointer, modifiers } : null,
        )
      : null
    if (custom !== null) return this.callbacks.onCursorChange(custom)
    if (tool !== 'select')
      return this.callbacks.onCursorChange(toolById(tool).cursor)
    this.callbacks.onCursorChange(cursorForHover(this.hover, modifiers))
  }

  /** Whether the active tool, not the controller, owns the pointer now. */
  private toolsOwnPointer(): boolean {
    return (
      this.tools.active && (this.tools.busy || this.effectiveTool() !== 'hand')
    )
  }

  /**
   * Alt-click converts a line segment to a curve and Shift-click inserts a
   * node on it (design spec §4.3); both are single commands, so they must not
   * open a drag gesture.
   */
  private handleSegmentShortcut(hit: Hit, modifiers: Modifiers): boolean {
    const target = this.layerTarget()
    if (!target) return false
    const action = segmentShortcut(
      target,
      this.labels,
      this.editOutline(),
      hit,
      modifiers,
    )
    if (!action) return false
    void this.runCommand(action.command, action.label)
    return true
  }

  private startDrag(hit: Hit, origin: Point2, modifiers: Modifiers): void {
    const target = this.layerTarget()
    if (!this.source || !target) return
    this.snapping.clear()
    const drag = startDragGesture(
      {
        target,
        contours: this.contours(),
        anchors: this.anchors(),
        components: this.components(),
        selection: this.selection,
        labels: this.labels,
        after: this.closing,
        onError: (error) => this.callbacks.onError(error),
        snap: (delta, moved) =>
          this.snapping.forDelta(this.snapSources(), delta, moved),
      },
      hit,
      origin,
      modifiers,
    )
    if (!drag) return
    this.drag = drag
    this.publishEditing()
  }

  private async finishDrag(): Promise<void> {
    const drag = this.drag
    if (!drag) return
    this.drag = null
    this.snapping.clear()
    let changed = true
    try {
      changed = await drag.finish()
    } catch (error) {
      this.callbacks.onError(error)
    } finally {
      // However the gesture ended, the preview goes away, reloads resume and
      // the canvas repaints; leaving any of those behind strands the editor.
      this.setPreview(null)
      this.publishEditing()
      // A gesture that failed part-way leaves the document in a state only the
      // kernel knows, so it is refetched even though nothing "changed".
      if (changed || drag.failed) this.callbacks.onReload()
      else this.callbacks.repaint()
    }
  }
}
