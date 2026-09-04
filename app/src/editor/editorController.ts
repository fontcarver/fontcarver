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

import type { Anchor, Contour, DocId, GlyphId, LayerOutline } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { EditingOverlay } from '../views/glyph/drawEditing'
import type { SceneMetrics } from '../views/glyph/glyphScene'

import { hudText, handlePolar, stemWidth, type HudLabels } from './hud'
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
import {
  marqueeRect,
  marqueeSelection,
  marqueeSize,
  type MarqueeRect,
} from './gestures/marquee'
import { NudgeCoalescer, nudgeStep } from './gestures/nudge'
import { buildEditOutline, pointAt, type EditOutline } from './outlineGeometry'
import {
  commit,
  convertSegmentCommand,
  deleteCommand,
  insertPointCommand,
  nextSmoothFlag,
  smoothCommand,
  type LayerTarget,
} from './pointCommands'
import { snapDelta, type SnapMatch } from './snapping'
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
  metrics: Readonly<
    Record<
      'baseline' | 'xHeight' | 'capHeight' | 'ascender' | 'descender',
      string
    >
  >
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
}

/** A pointer event, normalised to canvas space. */
export interface PointerInput {
  x: number
  y: number
  button: number
  modifiers: Modifiers
}

/** Everything the canvas needs from the controller to paint one frame. */
export interface EditorFrame {
  contours: readonly Contour[] | null
  anchors: readonly Anchor[] | null
  selected: ReadonlySet<string>
  overlay: EditingOverlay
  hud: string | null
  pointerScreen: Point2 | null
}

const EMPTY_FRAME_KEYS: ReadonlySet<string> = new Set()

/** How near the pointer must come to a component before its ghosts appear. */
const COMPONENT_GHOST_REACH = 24

/** Callbacks used until the view installs its own. */
const SILENT_CALLBACKS: EditorCallbacks = {
  repaint: () => undefined,
  onSelectionChange: () => undefined,
  onEditingChange: () => undefined,
  onZoomChange: () => undefined,
  onReload: () => undefined,
  onCursorChange: () => undefined,
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
  private spaceHeld = false
  private tool: ToolId = 'select'

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
    if (changedGlyph) this.setSelection(EMPTY_SELECTION)
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
    const overlay: EditingOverlay = {
      marquee: this.marqueeBox(),
      snap: this.snap?.point ?? null,
      alignX: this.snap?.alignedX ?? null,
      alignY: this.snap?.alignedY ?? null,
      insertGhost: this.insertGhost(),
      componentGhosts: this.componentGhosts(),
    }
    return {
      contours: this.preview?.contours ?? null,
      anchors: this.preview?.anchors ?? null,
      selected: this.selectionKeys(),
      overlay,
      hud: this.hud(),
      pointerScreen: this.pointer,
    }
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
    await commit(
      target,
      smoothCommand(target, nodes, smooth, !keepOutline),
      this.labels.toggleSmooth,
    )
    this.callbacks.onReload()
  }

  /** Deletes the selected nodes; `break` opens the contour instead. */
  async deleteSelection(breakContour: boolean): Promise<void> {
    const target = this.layerTarget()
    const points = selectedPointRefs(this.selection)
    if (!target || points.length === 0) return
    await commit(
      target,
      deleteCommand(
        target,
        points,
        breakContour ? 'BreakContour' : 'KeepShape',
      ),
      this.labels.deletePoints,
    )
    this.setSelection(EMPTY_SELECTION)
    this.callbacks.onReload()
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

  /** One arrow press. */
  nudgeSelection(direction: Point2, modifiers: Modifiers): void {
    const target = this.layerTarget()
    const points = selectedPointRefs(this.selection)
    if (!target || points.length === 0) return
    const step = nudgeStep(modifiers)
    this.nudge ??= new NudgeCoalescer({
      doc: target.doc,
      glyph: target.glyph,
      layer: target.layer,
      label: (count) => this.labels.nudgePoints(count),
    })
    this.callbacks.onEditingChange(true)
    this.nudge.nudge(
      points,
      { x: direction.x * step, y: direction.y * step },
      modifiers.alt ? 'PointsOnly' : 'WithHandles',
    )
  }

  /** Finalises a run of nudges: one undo entry, then a reload. */
  async endNudge(): Promise<void> {
    if (!this.nudge?.active) return
    await this.nudge.end()
    this.callbacks.onEditingChange(false)
    this.callbacks.onReload()
  }

  /** Escape: cancel a live drag, or clear the selection. */
  escape(): void {
    if (this.drag) {
      const drag = this.drag
      this.drag = null
      this.preview = null
      void drag.cancel().then(() => {
        this.callbacks.onEditingChange(false)
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

  private selectionKeys(): ReadonlySet<string> {
    if (this.selection.size === 0) return EMPTY_FRAME_KEYS
    return new Set(this.selection.keys())
  }

  private marqueeBox(): MarqueeRect | null {
    if (this.marquee)
      return marqueeRect(this.marquee.origin, this.marquee.current)
    if (this.zoomBox)
      return marqueeRect(this.zoomBox.origin, this.zoomBox.current)
    return null
  }

  private insertGhost(): Point2 | null {
    if (this.drag || this.marquee) return null
    return this.hover?.target.kind === 'segment' ? this.hover.point : null
  }

  private componentGhosts(): readonly Point2[] {
    const source = this.source
    if (!source || !this.pointer || this.drag) return []
    const font = this.viewport.toFont(this.pointer)
    const reach = COMPONENT_GHOST_REACH / (this.viewport.scale || 1)
    const ghosts: Point2[] = []
    for (const component of source.outline.components) {
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

  private hud(): string | null {
    const labels = this.labels.hud
    if (this.marquee || this.zoomBox) {
      const box = this.marqueeBox()
      if (!box) return null
      const scale = this.viewport.scale || 1
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
    const drag = this.drag
    if (!drag) return null
    if (drag.plan.kind === 'handle') {
      const handle = drag.plan.refs[0]
      const contours = this.contours()
      const point = handle ? pointAt(contours, handle) : null
      const node = handle
        ? this.edit.points.find(
            (candidate) =>
              candidate.onCurve &&
              candidate.ref.contour === handle.contour &&
              Math.abs(candidate.ref.point - handle.point) === 1,
          )
        : undefined
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
        stem: this.stem(),
        snap: drag.snappedTo,
      },
      labels,
    )
  }

  /** Distance between exactly two selected nodes, for the ⟂ readout. */
  private stem(): number | null {
    const nodes = selectedNodeRefs(this.selection)
    if (nodes.length !== 2) return null
    const contours = this.contours()
    const a = nodes[0] ? pointAt(contours, nodes[0]) : null
    const b = nodes[1] ? pointAt(contours, nodes[1]) : null
    return a && b ? stemWidth(a, b) : null
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
    if (modifiers.alt) {
      void commit(
        target,
        convertSegmentCommand(
          target,
          hit.target.contour,
          hit.target.segment,
          'Curve',
        ),
        this.labels.convertSegment,
      ).then(() => this.callbacks.onReload())
      return true
    }
    if (modifiers.shift) {
      void commit(
        target,
        insertPointCommand(
          target,
          hit.target.contour,
          hit.target.segment,
          Math.min(0.999, Math.max(0.001, hit.t ?? 0.5)),
        ),
        this.labels.insertPoint,
      ).then(() => this.callbacks.onReload())
      return true
    }
    return false
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
    this.callbacks.onEditingChange(true)
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
      metricLabels: this.labels.metrics,
      viewport: this.viewport,
      exclude,
    }
  }

  private async finishDrag(): Promise<void> {
    const drag = this.drag
    if (!drag) return
    this.drag = null
    this.snap = null
    const changed = await drag.finish()
    this.preview = null
    this.callbacks.onEditingChange(false)
    if (changed) this.callbacks.onReload()
    else this.callbacks.repaint()
  }
}
