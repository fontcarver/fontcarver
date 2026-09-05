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

import type { Anchor, Command, Contour } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'

import {
  allTargets,
  buildSnapInput,
  cycledSelection,
  segmentShortcut,
} from './editorActions'
import { buildFrame, type EditorFrame } from './editorFrame'
import {
  SILENT_CALLBACKS,
  type EditorCallbacks,
  type EditorLabels,
  type EditorSource,
  type PointerInput,
} from './editorTypes'
import { hitTest, type Hit, type HitTestInput } from './hitTest'
import {
  beginDrag,
  NO_MODIFIERS,
  type DragGesture,
  type Modifiers,
} from './gestures/drag'
import { marqueeSelection } from './gestures/marquee'
import { nudgeStep } from './gestures/nudge'
import { NudgeRunner } from './nudgeRun'
import { buildEditOutline, type EditOutline } from './outlineGeometry'
import {
  commit,
  deleteCommand,
  nextSmoothFlag,
  smoothCommand,
  type LayerTarget,
} from './pointCommands'
import { snapDelta, type SnapMatch } from './snapping'
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

export type {
  EditorCallbacks,
  EditorLabels,
  EditorSource,
  PointerInput,
} from './editorTypes'

export class EditorController {
  private readonly viewport: Viewport
  private callbacks: EditorCallbacks = SILENT_CALLBACKS
  private labels: EditorLabels
  private source: EditorSource | null = null
  private edit: EditOutline = buildEditOutline([])

  private selection: Selection = EMPTY_SELECTION
  private drag: DragGesture | null = null
  private readonly view = new ViewGestures()
  private hover: Hit | null = null
  private pointer: Point2 | null = null
  private snap: SnapMatch | null = null
  private preview: {
    contours: readonly Contour[]
    anchors: readonly Anchor[]
  } | null = null
  private readonly nudge: NudgeRunner
  private spaceHeld = false
  private tool: ToolId = 'select'
  private snapEnabled = true
  private editingPublished = false
  /** The gesture that is closing, if any; a new one begins after it. */
  private closing: Promise<void> = Promise.resolve()

  constructor(viewport: Viewport, labels: EditorLabels) {
    this.viewport = viewport
    this.labels = labels
    this.nudge = new NudgeRunner({
      onEnd: () => this.closeNudge(),
      onError: (error) => this.callbacks.onError(error),
    })
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
    const editing = this.drag !== null || this.nudge.active
    if (editing === this.editingPublished) return
    this.editingPublished = editing
    this.callbacks.onEditingChange(editing)
  }

  /** Records the optimistic geometry and tells the panels that read it. */
  private setPreview(
    preview: {
      contours: readonly Contour[]
      anchors: readonly Anchor[]
    } | null,
  ): void {
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
    this.snapEnabled = enabled
  }

  /** Which tool the strip currently has active (or borrowed). */
  setTool(tool: ToolId): void {
    void this.settle()
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
      edit: this.edit,
      selection: this.selection,
      components: this.source?.outline.components ?? [],
      pointer: this.pointer,
      drag: this.drag,
      band: this.view.band,
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
    if (this.effectiveTool() !== 'select') return

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
    void this.nudge.end()
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
    this.setSelection(allTargets(this.edit))
  }

  /** Tab and Shift+Tab walk the on-curve points in stored order. */
  cycleNodes(direction: 1 | -1): void {
    const next = cycledSelection(this.edit, this.selection, direction)
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
    const step = nudgeStep(modifiers)
    this.setPreview(
      this.nudge.press({
        target,
        contours: source.outline.contours,
        anchors: source.outline.anchors,
        refs: points,
        delta: { x: direction.x * step, y: direction.y * step },
        handles: modifiers.alt ? 'PointsOnly' : 'WithHandles',
        label: (count) => this.labels.nudgePoints(count),
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
   * Closes any coalesced run that is still open and reports when it is done.
   *
   * `doc.edit_begin` and `doc.apply` both finalise the active edit in the
   * kernel, so a gesture that starts while a nudge run is still flushing would
   * invalidate that run's last apply — the arrow press would be dropped and an
   * error raised. Every new gesture waits for this first.
   */
  settle(): Promise<void> {
    if (this.nudge.active) this.closing = this.nudge.end()
    return this.closing
  }

  /**
   * A nudge run closed, from the idle timer or from any other action. The
   * preview is handed back to the layer and reloads resume.
   */
  private closeNudge(): void {
    this.setPreview(null)
    this.publishEditing()
    this.callbacks.onReload()
    this.callbacks.repaint()
  }

  /** Escape: cancel a live drag, or clear the selection. */
  escape(): void {
    if (this.drag) {
      const drag = this.drag
      this.drag = null
      this.setPreview(null)
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
        this.view.panningNow ? HAND_DRAGGING_CURSOR : HAND_CURSOR,
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
    if (!target) return false
    const action = segmentShortcut(
      target,
      this.labels,
      this.edit,
      hit,
      modifiers,
    )
    if (!action) return false
    void this.runCommand(action.command, action.label)
    return true
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
        after: this.closing,
        labels: {
          movePoints: (count) => this.labels.movePoints(count),
          moveHandles: (count) => this.labels.moveHandles(count),
          moveHandle: this.labels.moveHandle,
          reshapeSegment: this.labels.reshapeSegment,
          moveAnchor: this.labels.moveAnchor,
          moveComponent: this.labels.moveComponent,
        },
        onError: (error) => this.callbacks.onError(error),
        snap: (delta, moved) => {
          if (!this.snapEnabled) {
            this.snap = null
            return { delta, label: null }
          }
          const match = snapDelta(
            buildSnapInput({
              outline: this.edit,
              source: this.source,
              selection: this.selection,
              drag: this.drag,
              labels: this.labels,
              viewport: this.viewport,
            }),
            delta,
            moved,
          )
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
      this.setPreview(null)
      this.publishEditing()
      // A gesture that failed part-way leaves the document in a state only the
      // kernel knows, so it is refetched even though nothing "changed".
      if (changed || drag.failed) this.callbacks.onReload()
      else this.callbacks.repaint()
    }
  }
}
