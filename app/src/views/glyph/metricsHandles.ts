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
 *
 * The two *horizontal* lines of a CJK font — the vertical origin and the
 * vertical advance below it — are grabbable the same way, hit-tested on y and
 * written with the equally absolute `SetVerticalMetrics`. A vertical drag
 * carries the layer's *stored* value for the field it does not own, so pulling
 * the origin of a glyph that inherits its advance from the master leaves that
 * inheritance alone instead of freezing a copy of it.
 */

import type { Command, DocId, LayerOutline } from '../../api'
import type { Point2, Viewport } from '../../canvas/Viewport'
import { composeAffine, type Affine } from '../../editor/affine'
import { EditSession, type Scheduler } from '../../editor/editSession'
import type { EditorPreview } from '../../editor/editorTypes'

import type { SceneMetrics } from './glyphScene'

/** Which metric line the pointer grabbed. */
export type MetricHandle = 'lsb' | 'rsb' | 'width' | 'vorigin' | 'vadvance'

/** Grab radius of a metric line, in CSS pixels; the same 4 px a guide gets. */
export const METRIC_HIT = 4

/** Cursor shown over a vertical metric line. */
export const METRIC_CURSOR = 'ew-resize'

/** Cursor shown over a horizontal (vertical-writing) metric line. */
export const VERTICAL_METRIC_CURSOR = 'ns-resize'

/** The horizontal lines, which are hit-tested and dragged along y. */
const VERTICAL_HANDLES: readonly MetricHandle[] = ['vorigin', 'vadvance']

/** OpenType limits the kernel enforces, applied while dragging so a whole
 * gesture stays inside them instead of failing at the far end of a drag. */
const ORIGIN_RANGE: readonly [number, number] = [-32768, 32767]
const ADVANCE_RANGE: readonly [number, number] = [0, 65535]

/** Which axis a handle lives on: `x` for a vertical line, `y` for a horizontal one. */
export function metricHandleAxis(handle: MetricHandle): 'x' | 'y' {
  return VERTICAL_HANDLES.includes(handle) ? 'y' : 'x'
}

/** The cursor to show over one handle. */
export function metricCursor(handle: MetricHandle): string {
  return metricHandleAxis(handle) === 'y'
    ? VERTICAL_METRIC_CURSOR
    : METRIC_CURSOR
}

/** The `glyphView.metric.*` key naming each handle in the HUD. */
export const METRIC_LABEL_KEYS: Record<MetricHandle, string> = {
  lsb: 'glyphView.metric.lsb',
  rsb: 'glyphView.metric.rsb',
  width: 'glyphView.metric.advance',
  vorigin: 'glyphView.metric.verticalOrigin',
  vadvance: 'glyphView.metric.verticalAdvance',
}

function clamp(value: number, [low, high]: readonly [number, number]): number {
  return Math.min(high, Math.max(low, value))
}

/** The font-space x each draggable vertical sits at, or `null` when it has none. */
export function metricLineX(
  metrics: SceneMetrics,
  handle: MetricHandle,
): number | null {
  if (handle === 'width') return metrics.width
  if (handle === 'lsb') return metrics.lsb
  if (handle === 'rsb')
    return metrics.rsb === null ? null : metrics.width - metrics.rsb
  return null
}

/**
 * The font-space y each draggable horizontal sits at, or `null`.
 *
 * These are exactly the lines `drawMetrics` paints for a vertical-writing
 * font: the origin, and the advance one advance height below it. A font with no
 * vertical metrics has neither line and therefore neither handle.
 */
export function metricLineY(
  metrics: SceneMetrics,
  handle: MetricHandle,
): number | null {
  const origin = metrics.verticalOrigin
  if (origin === null) return null
  if (handle === 'vorigin') return origin
  if (handle !== 'vadvance') return null
  return metrics.verticalWidth === null ? null : origin - metrics.verticalWidth
}

/**
 * The metric handle under a canvas point, or `null`.
 *
 * The advance line is tested last: on a glyph whose right sidebearing is zero
 * the two coincide, and "drag the sidebearing" is the more useful reading of a
 * click there — the advance still follows, because moving the RSB moves it.
 *
 * The horizontal lines are offered **only in vertical writing mode**, and then
 * only when no sidebearing or advance line was grabbed. They run the whole
 * width of the canvas and outrank segments and components, so in ordinary
 * horizontal editing of a CJK font — where the master's default vertical origin
 * gives every glyph those lines — they would otherwise put two full-width bands
 * in front of the outline. Turning vertical mode on is the deliberate act that
 * makes them grabbable.
 */
export function metricsHit(
  metrics: SceneMetrics,
  viewport: Viewport,
  screen: Point2,
  verticalMode = false,
): MetricHandle | null {
  let best: { handle: MetricHandle; distance: number } | null = null
  for (const handle of ['lsb', 'rsb', 'width'] as const) {
    const x = metricLineX(metrics, handle)
    if (x === null) continue
    const distance = Math.abs(viewport.toScreen({ x, y: 0 }).x - screen.x)
    if (distance > METRIC_HIT) continue
    if (!best || distance < best.distance) best = { handle, distance }
  }
  if (best || !verticalMode) return best?.handle ?? null
  for (const handle of ['vorigin', 'vadvance'] as const) {
    const y = metricLineY(metrics, handle)
    if (y === null) continue
    const distance = Math.abs(viewport.toScreen({ x: 0, y }).y - screen.y)
    if (distance > METRIC_HIT) continue
    if (!best || distance < best.distance) best = { handle, distance }
  }
  return best?.handle ?? null
}

/** The layer's metrics at the moment a drag started. */
export interface StartMetrics {
  width: number
  lsb: number
  rsb: number
  /** Effective vertical origin, `null` in a font with no vertical metrics. */
  verticalOrigin?: number | null
  /** Effective vertical advance, `null` when the font declares none. */
  verticalWidth?: number | null
}

/** The layer's *stored* vertical overrides, `null` where it inherits. */
export interface StoredVerticalMetrics {
  origin: number | null
  advance: number | null
}

/**
 * The scene metrics a live gesture is previewing.
 *
 * Only the fields a drag actually changed replace the scene's own, so a
 * horizontal drag never blanks the vertical lines and the reverse.
 */
export function withPreview(
  metrics: SceneMetrics,
  preview: StartMetrics,
): SceneMetrics {
  return {
    ...metrics,
    width: preview.width,
    lsb: preview.lsb,
    rsb: preview.rsb,
    verticalOrigin:
      preview.verticalOrigin === undefined
        ? metrics.verticalOrigin
        : preview.verticalOrigin,
    verticalWidth:
      preview.verticalWidth === undefined
        ? metrics.verticalWidth
        : preview.verticalWidth,
  }
}

/** What a metrics drag needs. */
export interface MetricsDragOptions {
  doc: DocId
  glyph: string
  layer: string
  handle: MetricHandle
  /** Font-space position of the pointer when it went down, along the handle's axis. */
  origin: number
  start: StartMetrics
  /** The layer's stored vertical overrides, for a vertical-handle drag. */
  stored?: StoredVerticalMetrics
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
  if (handle === 'vorigin' || handle === 'vadvance') {
    // The advance line sits *below* the origin, so pulling it down lengthens
    // the advance; the origin simply follows the pointer. Both are clamped to
    // the ranges the kernel accepts so a long drag cannot be rejected.
    const raw =
      handle === 'vorigin'
        ? (start.verticalOrigin ?? 0) + delta
        : (start.verticalWidth ?? 0) - delta
    return clamp(
      Math.round(raw / step) * step,
      handle === 'vorigin' ? ORIGIN_RANGE : ADVANCE_RANGE,
    )
  }
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
    if (this.handle === 'vorigin')
      return { ...start, verticalOrigin: this.current }
    if (this.handle === 'vadvance')
      return { ...start, verticalWidth: this.current }
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
    // Width-only edits leave ink fixed, so the right bearing absorbs the
    // advance delta, including a finite negative advance accepted by the kernel.
    return {
      ...start,
      width: this.current,
      rsb: start.rsb + (this.current - start.width),
    }
  }

  /** Geometry the canvas should draw while this metrics edit is live. */
  get geometryPreview(): EditorPreview {
    const delta =
      this.handle === 'lsb' ? this.current - this.options.start.lsb : 0
    return metricsGeometryPreview(this.options.outline, delta)
  }

  /**
   * The pointer moved; `coarse` is the ⇧ modifier's ten-unit step.
   *
   * A horizontal handle needs `fontY`; without it the move is ignored rather
   * than read off the wrong axis.
   */
  move(fontX: number, coarse: boolean, fontY?: number): void {
    const along = metricHandleAxis(this.handle) === 'y' ? fontY : fontX
    if (along === undefined) return
    const next = metricValue(
      this.handle,
      this.options.start,
      along - this.options.origin,
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
    const { glyph, layer, stored } = this.options
    if (this.handle === 'vorigin' || this.handle === 'vadvance') {
      // `SetVerticalMetrics` writes both fields, so the one this drag does not
      // own is sent exactly as the layer stores it — `null` included, which is
      // how the layer says "inherit the master default".
      return {
        SetVerticalMetrics: {
          glyph,
          layer,
          advance:
            this.handle === 'vadvance'
              ? this.current
              : (stored?.advance ?? null),
          origin:
            this.handle === 'vorigin' ? this.current : (stored?.origin ?? null),
        },
      }
    }
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
