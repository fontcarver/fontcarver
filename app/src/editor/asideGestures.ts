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
  private ruler: RulerGuideDrag | null = null

  constructor(context: AsideContext) {
    this.context = context
  }

  /** Whether a gesture that changes the document is live. */
  get editing(): boolean {
    return this.metrics !== null
  }

  /** Whether any of these gestures currently owns the pointer. */
  get active(): boolean {
    return this.metrics !== null || this.ruler !== null
  }

  /** The cursor to show over a metric line, or `null` for none. */
  cursorAt(at: Point2 | null): string | null {
    return this.handleAt(at) === null ? null : METRIC_CURSOR
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
    void gesture
      .cancel()
      .catch((error: unknown) => this.context.onError(error))
      .finally(() => {
        this.context.publishEditing()
        this.context.onReload()
      })
    this.context.repaint()
    return true
  }

  /** The view is going away: close whatever is open. */
  dispose(): void {
    void this.metrics?.cancel()
    this.metrics = null
    this.ruler = null
  }

  /** What these gestures contribute to the painted frame. */
  frame(): FrameState['aside'] {
    if (this.metrics)
      return {
        hud: this.context
          .labels()
          .metricHud(this.metrics.handle, this.metrics.value),
        metrics: this.metrics.preview,
      }
    if (this.ruler)
      return { alignX: this.ruler.previewX, alignY: this.ruler.previewY }
    return null
  }

  private handleAt(at: Point2 | null) {
    const source = this.context.source()
    if (!at || !source) return null
    return metricsHit(source.metrics, this.context.viewport, at)
  }

  private startMetrics(at: Point2): boolean {
    const target = this.context.target()
    const source = this.context.source()
    const handle = this.handleAt(at)
    if (!target || !source || !handle) return false
    this.metrics = new MetricsDrag({
      doc: target.doc,
      glyph: target.glyph,
      layer: target.layer,
      handle,
      origin: this.context.viewport.toFont(at).x,
      start: {
        width: source.outline.width,
        lsb: source.outline.lsb,
        rsb: source.outline.rsb,
      },
      label: this.context.labels().setMetrics,
      after: this.context.after(),
      onError: this.context.onError,
    })
    this.context.publishEditing()
    this.context.repaint()
    return true
  }

  private async finishMetrics(): Promise<void> {
    const gesture = this.metrics
    if (!gesture) return
    this.metrics = null
    let changed = true
    try {
      changed = await gesture.finish()
    } catch (error) {
      this.context.onError(error)
    } finally {
      this.context.publishEditing()
      // A gesture that failed part-way leaves the document in a state only the
      // kernel knows, so it is refetched even though nothing "changed".
      if (changed || gesture.failed) this.context.onReload()
      else this.context.repaint()
    }
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
