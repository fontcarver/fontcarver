/**
 * The two gestures that are not the drag/marquee pair: pulling a sidebearing
 * or advance line, and dragging a guide out of a ruler.
 *
 * Both are small, both are complete in themselves, and both would otherwise
 * double the controller's pointer routing. Keeping them together behind one
 * object lets the controller ask three questions — "did this pointer event
 * belong to you?", "are you editing?", "what do you contribute to the frame?"
 * — and stay about the node grammar it exists for.
 */

import type { Point2, Viewport } from '../canvas/Viewport'
import {
  METRIC_CURSOR,
  MetricsDrag,
  metricsHit,
} from '../views/glyph/metricsHandles'
import {
  RulerGuideDrag,
  shouldStartRulerGuide,
} from '../views/glyph/rulerGuides'

import type { FrameState } from './editorFrame'
import type { EditorLabels, EditorSource } from './editorTypes'
import type { Modifiers } from './gestures/drag'
import type { LayerTarget } from './pointCommands'

/** Everything the aside gestures borrow from the controller. */
export interface AsideContext {
  viewport: Viewport
  /** The layer on screen, or `null` before one has loaded. */
  source: () => EditorSource | null
  /** The glyph and layer a command names, or `null`. */
  target: () => LayerTarget | null
  labels: () => EditorLabels
  /** Whether the rulers are drawn; ⌥-dragging one only works when they are. */
  rulersVisible: () => boolean
  /** A gesture still closing, which a new one's `edit_begin` waits for. */
  after: () => Promise<unknown> | undefined
  /** Closes any coalesced run before a single-shot `doc.apply`. */
  settle: () => Promise<void>
  /** Registers a closing gesture with the controller's shared edit gate. */
  gate: (closing: Promise<unknown>) => Promise<void>
  onError: (error: unknown) => void
  onReload: () => void
  repaint: () => void
  /** The controller re-derives and republishes its editing flag. */
  publishEditing: () => void
}

/** The sidebearing and ruler gestures of one editor. */
export class AsideGestures {
  private readonly context: AsideContext
  private metrics: MetricsDrag | null = null
  private metricsKey: string | null = null
  private closingMetrics: {
    gesture: MetricsDrag
    key: string
    cancel: boolean
  } | null = null
  /** Last committed preview, retained until the authoritative reload lands. */
  private settledMetrics: { gesture: MetricsDrag; key: string } | null = null
  private metricClosures = 0
  private ruler: RulerGuideDrag | null = null

  constructor(context: AsideContext) {
    this.context = context
  }

  /** Whether a gesture that changes the document is live. */
  get editing(): boolean {
    return this.metrics !== null || this.metricClosures > 0
  }

  /** Whether any of these gestures currently owns the pointer. */
  get active(): boolean {
    return this.metrics !== null || this.ruler !== null
  }

  /** The cursor to show over a metric line, or `null` for none. */
  cursorAt(at: Point2 | null): string | null {
    return this.handleAt(at) === null ? null : METRIC_CURSOR
  }

  /** New kernel data has reconciled any optimistic completed metrics edit. */
  sourceChanged(): void {
    this.settledMetrics = null
  }

  /**
   * Offers a pointer-down. `overGrabbable` says whether the hit test found
   * something that outranks a metric line — design spec §4.2 puts nodes,
   * handles, anchors and guides ahead of it and segments and components
   * behind, because a sidebearing runs the whole height of the canvas and
   * would otherwise be unreachable wherever it crosses the outline.
   */
  pointerDown(
    at: Point2,
    modifiers: Modifiers,
    overGrabbable: boolean,
  ): boolean {
    if (this.startRuler(at, modifiers)) return true
    if (overGrabbable) return false
    return this.startMetrics(at)
  }

  /** Offers a pointer move; reports whether a gesture consumed it. */
  pointerMove(at: Point2, modifiers: Modifiers): boolean {
    const font = this.context.viewport.toFont(at)
    if (this.metrics) {
      this.metrics.move(font.x, modifiers.shift)
      this.context.repaint()
      return true
    }
    if (!this.ruler) return false
    this.ruler.move(font)
    this.context.repaint()
    return true
  }

  /** Offers a pointer-up; reports whether a gesture consumed it. */
  pointerUp(): boolean {
    if (this.metrics) {
      void this.finishMetrics()
      return true
    }
    if (!this.ruler) return false
    void this.finishRuler()
    return true
  }

  /** Escape: abandons whichever gesture is live. Reports whether one was. */
  escape(): boolean {
    if (this.ruler) {
      this.ruler = null
      this.context.repaint()
      return true
    }
    const gesture = this.metrics
    if (!gesture) return false
    this.metrics = null
    void this.closeMetrics(gesture, true)
    this.context.repaint()
    return true
  }

  /** Finishes an abandoned live metrics gesture before another edit starts. */
  settle(): Promise<void> | null {
    return this.metrics ? this.finishMetrics() : null
  }

  /** The view is going away: close whatever is open. */
  dispose(): void {
    if (this.metrics) void this.closeMetrics(this.metrics, true)
    this.metrics = null
    this.ruler = null
  }

  /** What these gestures contribute to the painted frame. */
  frame(): FrameState['aside'] {
    const source = this.context.source()
    const key = source ? this.sourceKey(source) : null
    const closing =
      this.closingMetrics?.key === key && !this.closingMetrics.cancel
        ? this.closingMetrics.gesture
        : null
    const settled =
      this.settledMetrics?.key === key ? this.settledMetrics.gesture : null
    const metrics = this.metrics ?? closing ?? settled
    if (metrics)
      return {
        hud: this.context.labels().metricHud(metrics.handle, metrics.value),
        metrics: metrics.preview,
        preview: metrics.geometryPreview,
      }
    if (this.ruler)
      return { alignX: this.ruler.previewX, alignY: this.ruler.previewY }
    return null
  }

  private handleAt(at: Point2 | null) {
    const source = this.context.source()
    if (!at || !source) return null
    const prior = this.priorMetrics(source)
    return metricsHit(
      prior ? { ...source.metrics, ...prior.preview } : source.metrics,
      this.context.viewport,
      at,
    )
  }

  private startMetrics(at: Point2): boolean {
    const target = this.context.target()
    const source = this.context.source()
    const handle = this.handleAt(at)
    if (!target || !source || !handle) return false
    const prior = this.priorMetrics(source)
    this.metrics = new MetricsDrag({
      doc: target.doc,
      glyph: target.glyph,
      layer: target.layer,
      handle,
      origin: this.context.viewport.toFont(at).x,
      start: prior
        ? prior.preview
        : {
            width: source.outline.width,
            lsb: source.outline.lsb,
            rsb: source.outline.rsb,
          },
      outline: prior
        ? {
            ...source.outline,
            contours: [...prior.geometryPreview.contours],
            anchors: [...prior.geometryPreview.anchors],
            components: [...prior.geometryPreview.components],
          }
        : source.outline,
      label: this.context.labels().setMetrics,
      after: this.context.after(),
      onError: this.context.onError,
    })
    this.metricsKey = this.sourceKey(source)
    this.context.publishEditing()
    this.context.repaint()
    return true
  }

  private finishMetrics(): Promise<void> {
    const gesture = this.metrics
    if (!gesture) return Promise.resolve()
    this.metrics = null
    return this.closeMetrics(gesture, false)
  }

  private closeMetrics(gesture: MetricsDrag, cancel: boolean): Promise<void> {
    const key = this.metricsKey ?? this.currentSourceKey()
    this.metricsKey = null
    this.closingMetrics = { gesture, key, cancel }
    this.metricClosures += 1
    this.context.publishEditing()
    const closing = (async () => {
      let changed = true
      try {
        if (cancel) await gesture.cancel()
        else changed = await gesture.finish()
      } catch (error) {
        this.context.onError(error)
      } finally {
        this.metricClosures -= 1
        if (this.closingMetrics?.gesture === gesture) this.closingMetrics = null
        if (!cancel && changed && !gesture.failed)
          this.settledMetrics = { gesture, key }
        this.context.publishEditing()
        // A gesture that failed part-way leaves the document in a state only
        // the kernel knows, so it is refetched even without a complete edit.
        if (cancel || changed || gesture.failed) this.context.onReload()
        else this.context.repaint()
      }
    })()
    return this.context.gate(closing)
  }

  private priorMetrics(source: EditorSource): MetricsDrag | null {
    const key = this.sourceKey(source)
    if (this.closingMetrics?.key === key && !this.closingMetrics.cancel)
      return this.closingMetrics.gesture
    return this.settledMetrics?.key === key ? this.settledMetrics.gesture : null
  }

  private currentSourceKey(): string {
    const source = this.context.source()
    return source ? this.sourceKey(source) : ''
  }

  private sourceKey(source: EditorSource): string {
    return JSON.stringify([source.doc, source.glyph, source.outline.layer])
  }

  private startRuler(at: Point2, modifiers: Modifiers): boolean {
    const target = this.context.target()
    const source = this.context.source()
    const axis = shouldStartRulerGuide(
      at,
      modifiers,
      this.context.rulersVisible(),
    )
    if (!target || !source || !axis) return false
    this.ruler = new RulerGuideDrag(
      {
        doc: target.doc,
        glyph: target.glyph,
        layer: target.layer,
        master: source.master,
        // ⇧ puts the guide in the master's global list, ⌥ alone in this layer's.
        scope: modifiers.shift ? 'master' : 'layer',
        axis,
        label: this.context.labels().addGuide,
      },
      this.context.viewport.toFont(at),
    )
    this.context.repaint()
    return true
  }

  private async finishRuler(): Promise<void> {
    const gesture = this.ruler
    if (!gesture) return
    this.ruler = null
    await this.context.settle()
    try {
      await gesture.finish()
    } catch (error) {
      this.context.onError(error)
    } finally {
      this.context.onReload()
      this.context.repaint()
    }
  }
}
