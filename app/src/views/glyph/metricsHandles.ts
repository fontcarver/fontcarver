/**
 * Dragging the sidebearing and advance lines on the canvas (design spec §7.2).
 *
 * The metric verticals `drawMetrics` paints — the origin, the advance and the
 * two sidebearing lines — are grabbable: pulling one is the same edit the
 * Metrics panel's fields make, and it goes through the same
 * `edit_begin`/`edit_apply`/`edit_end` protocol every other gesture uses, so a
 * whole drag is one undo entry.
 *
 * `SetMetrics` is *absolute* — it moves the outline so the sidebearing becomes
 * the value given — which makes each frame's command idempotent: sending the
 * same value twice changes nothing the second time. That is why this gesture
 * sends the value rather than a delta the way `MovePoints` has to.
 */

import type { Command, DocId, LayerOutline } from '../../api'
import type { Point2, Viewport } from '../../canvas/Viewport'
import { composeAffine, type Affine } from '../../editor/affine'
import { EditSession, type Scheduler } from '../../editor/editSession'
import type { EditorPreview } from '../../editor/editorTypes'

import type { SceneMetrics } from './glyphScene'

/** Which vertical the pointer grabbed. */
export type MetricHandle = 'lsb' | 'rsb' | 'width'

/** Grab radius of a metric line, in CSS pixels; the same 4 px a guide gets. */
export const METRIC_HIT = 4

/** Cursor shown over a metric handle. */
export const METRIC_CURSOR = 'ew-resize'

/** The font-space x each draggable vertical sits at, or `null` when it has none. */
export function metricLineX(
  metrics: SceneMetrics,
  handle: MetricHandle,
): number | null {
  if (handle === 'width') return metrics.width
  if (handle === 'lsb') return metrics.lsb
  return metrics.rsb === null ? null : metrics.width - metrics.rsb
}

/**
 * The metric handle under a canvas point, or `null`.
 *
 * The advance line is tested last: on a glyph whose right sidebearing is zero
 * the two coincide, and "drag the sidebearing" is the more useful reading of a
 * click there — the advance still follows, because moving the RSB moves it.
 */
export function metricsHit(
  metrics: SceneMetrics,
  viewport: Viewport,
  screen: Point2,
): MetricHandle | null {
  let best: { handle: MetricHandle; distance: number } | null = null
  for (const handle of ['lsb', 'rsb', 'width'] as const) {
    const x = metricLineX(metrics, handle)
    if (x === null) continue
    const distance = Math.abs(viewport.toScreen({ x, y: 0 }).x - screen.x)
    if (distance > METRIC_HIT) continue
    if (!best || distance < best.distance) best = { handle, distance }
  }
  return best?.handle ?? null
}

/** The layer's horizontal metrics at the moment a drag started. */
export interface StartMetrics {
  width: number
  lsb: number
  rsb: number
}

/** What a metrics drag needs. */
export interface MetricsDragOptions {
  doc: DocId
  glyph: string
  layer: string
  handle: MetricHandle
  /** Font-space x of the pointer when it went down. */
  origin: number
  start: StartMetrics
  /** Stored geometry translated by an LSB edit exactly as the kernel does. */
  outline: LayerOutline
  /** Undo label, already translated. */
  label: string
  scheduler?: Scheduler
  onError?: (error: unknown) => void
  /** A gesture still closing, which this one's `edit_begin` waits for. */
  after?: Promise<unknown>
}

/** The value one handle takes when the pointer has moved `delta` font units. */
export function metricValue(
  handle: MetricHandle,
  start: StartMetrics,
  delta: number,
  step: number,
): number {
  // The right sidebearing shrinks as its line moves right, which is the
  // opposite sense to the other two.
  const raw =
    handle === 'lsb'
      ? start.lsb + delta
      : handle === 'rsb'
        ? start.rsb - delta
        : start.width + delta
  return Math.round(raw / step) * step
}

/** One live sidebearing or advance drag. */
export class MetricsDrag {
  readonly handle: MetricHandle
  private readonly options: MetricsDragOptions
  private readonly session: EditSession
  private current: number
  private sent: number | null = null

  constructor(options: MetricsDragOptions) {
    this.options = options
    this.handle = options.handle
    this.current = metricValue(options.handle, options.start, 0, 1)
    this.session = new EditSession(options.doc, options.label, {
      scheduler: options.scheduler,
      onError: options.onError,
      after: options.after,
    })
  }

  /** The value the handle currently shows, for the HUD and the preview. */
  get value(): number {
    return this.current
  }

  /** Whether any call of this gesture failed. */
  get failed(): boolean {
    return this.session.failed
  }

  /** The metrics the canvas should draw while the gesture runs. */
  get preview(): StartMetrics {
    const { start } = this.options
    if (this.handle === 'lsb')
      // Moving the left sidebearing shifts the outline, so the advance grows
      // by the same amount and the right sidebearing is unchanged.
      return {
        ...start,
        lsb: this.current,
        width: start.width + (this.current - start.lsb),
      }
    if (this.handle === 'rsb')
      return {
        ...start,
        rsb: this.current,
        width: start.width + (this.current - start.rsb),
      }
    return { ...start, width: this.current }
  }

  /** Geometry the canvas should draw while this metrics edit is live. */
  get geometryPreview(): EditorPreview {
    const delta =
      this.handle === 'lsb' ? this.current - this.options.start.lsb : 0
    return metricsGeometryPreview(this.options.outline, delta)
  }

  /** The pointer moved; `coarse` is the ⇧ modifier's ten-unit step. */
  move(fontX: number, coarse: boolean): void {
    const next = metricValue(
      this.handle,
      this.options.start,
      fontX - this.options.origin,
      coarse ? 10 : 1,
    )
    if (next === this.current) return
    this.current = next
    this.session.queue(() => this.command())
  }

  /** Finishes the gesture; resolves to whether anything was applied. */
  async finish(): Promise<boolean> {
    await this.session.end()
    return this.session.dirty
  }

  /** Abandons the gesture, undoing whatever already reached the kernel. */
  cancel(): Promise<void> {
    return this.session.cancel()
  }

  private command(): Command | null {
    if (this.sent === this.current) return null
    this.sent = this.current
    const { glyph, layer } = this.options
    return {
      SetMetrics: {
        glyph,
        layer,
        width: this.handle === 'width' ? this.current : null,
        lsb: this.handle === 'lsb' ? this.current : null,
        rsb: this.handle === 'rsb' ? this.current : null,
      },
    }
  }
}

/**
 * Applies the geometry half of `SetMetrics { lsb }` optimistically.
 *
 * The kernel left-multiplies every component by the translation, moves every
 * contour point and anchor, and leaves geometry alone for RSB/width edits.
 */
export function metricsGeometryPreview(
  outline: LayerOutline,
  delta: number,
): EditorPreview {
  const translation: Affine = [1, 0, 0, 1, delta, 0]
  return {
    contours: outline.contours.map((contour) => ({
      ...contour,
      points: contour.points.map((point) => ({
        ...point,
        x: point.x + delta,
      })),
    })),
    anchors: outline.anchors.map((anchor) => ({
      ...anchor,
      x: anchor.x + delta,
    })),
    components: outline.components.map((view) => ({
      ...view,
      component: {
        ...view.component,
        transform: composeAffine(
          translation,
          view.component.transform as Affine,
        ),
      },
      bbox: view.bbox
        ? [
            view.bbox[0] + delta,
            view.bbox[1],
            view.bbox[2] + delta,
            view.bbox[3],
          ]
        : null,
    })),
  }
}
