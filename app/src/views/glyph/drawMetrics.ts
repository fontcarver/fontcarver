/**
 * Metric lines: the vertical metrics of the master, the glyph's origin and
 * advance, its sidebearings, and the vertical origin and advance that CJK
 * fonts carry.
 *
 * Lines span the whole canvas and are snapped to half pixels so they stay one
 * device pixel wide at any zoom. Labels are drawn in screen space, pinned to
 * the left edge, because a label that scales with the outline is unreadable.
 */

import type { Viewport } from '../../canvas/Viewport'

import type { SceneMetrics } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'

/**
 * Left inset of the metric labels, in CSS pixels. Wide enough to clear the
 * vertical ruler, which is painted over the metrics.
 */
const LABEL_INSET = 24
const LABEL_FONT_SIZE = 10

/** Labels nearer the left edge than this would be hidden by the ruler. */
const RULER_LABEL_MARGIN = 20

/** One horizontal metric line. */
interface MetricLine {
  y: number
  label: string
  style: 'regular' | 'strong' | 'vertical'
}

/** Coincident metric lines share one stroke and one composite label. */
interface MetricGroup {
  screenY: number
  labels: string[]
  strong: boolean
  vertical: boolean
}

function crisp(value: number): number {
  return Math.round(value) + 0.5
}

function line(
  ctx: CanvasRenderingContext2D,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): void {
  ctx.beginPath()
  ctx.moveTo(x0, y0)
  ctx.lineTo(x1, y1)
  ctx.stroke()
}

/** Labels for the horizontal metric lines, in the order they are drawn. */
export interface MetricLabels {
  baseline: string
  xHeight: string
  capHeight: string
  ascender: string
  descender: string
  origin: string
  advance: string
  lsb: string
  rsb: string
  verticalOrigin: string
  verticalAdvance: string
}

/** Placeholder labels used when no translations are supplied. */
export const DEFAULT_METRIC_LABELS: MetricLabels = {
  baseline: 'baseline',
  xHeight: 'x-height',
  capHeight: 'cap height',
  ascender: 'ascender',
  descender: 'descender',
  origin: 'origin',
  advance: 'advance',
  lsb: 'LSB',
  rsb: 'RSB',
  verticalOrigin: 'vert. origin',
  verticalAdvance: 'vert. advance',
}

/** What `drawMetrics` renders. */
export interface MetricsData {
  metrics: SceneMetrics
  labels: MetricLabels
  /** Draw the metric names and values next to each line. */
  showLabels: boolean
}

/** Draws the metric lines of one glyph layer. */
export function drawMetrics(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: MetricsData,
  theme: GlyphTheme,
): void {
  const { metrics, labels } = data
  // A metric a font does not define comes back as zero, which would draw a
  // second line on top of the baseline; those are dropped rather than drawn.
  const baseHorizontals: MetricLine[] = [
    { y: metrics.descender, label: labels.descender, style: 'regular' },
    { y: 0, label: labels.baseline, style: 'strong' },
    { y: metrics.xHeight, label: labels.xHeight, style: 'regular' },
    { y: metrics.capHeight, label: labels.capHeight, style: 'regular' },
    { y: metrics.ascender, label: labels.ascender, style: 'regular' },
  ]
  const horizontals = baseHorizontals.filter(
    (metric) => metric.style === 'strong' || metric.y !== 0,
  )
  if (metrics.verticalOrigin !== null) {
    horizontals.push({
      y: metrics.verticalOrigin,
      label: labels.verticalOrigin,
      style: 'vertical',
    })
    if (metrics.verticalWidth !== null)
      horizontals.push({
        y: metrics.verticalOrigin - metrics.verticalWidth,
        label: labels.verticalAdvance,
        style: 'vertical',
      })
  }

  ctx.save()
  ctx.lineWidth = 1
  ctx.font = `${LABEL_FONT_SIZE}px ${theme.fontFamily}`
  ctx.textBaseline = 'bottom'
  ctx.textAlign = 'left'

  const groups = new Map<number, MetricGroup>()
  for (const metric of horizontals) {
    const screenY = crisp(viewport.toScreen({ x: 0, y: metric.y }).y)
    if (screenY < -1 || screenY > viewport.height + 1) continue
    const label = `${metric.label} ${Math.round(metric.y)}`
    const group = groups.get(screenY)
    if (group) {
      group.labels.push(label)
      group.strong ||= metric.style === 'strong'
      group.vertical ||= metric.style === 'vertical'
    } else {
      groups.set(screenY, {
        screenY,
        labels: [label],
        strong: metric.style === 'strong',
        vertical: metric.style === 'vertical',
      })
    }
  }

  for (const metric of groups.values()) {
    ctx.strokeStyle = metric.vertical
      ? theme.sidebearing
      : metric.strong
        ? theme.metricStrong
        : theme.metric
    ctx.setLineDash(metric.vertical ? [6, 3] : [])
    line(ctx, 0, metric.screenY, viewport.width, metric.screenY)
    ctx.setLineDash([])
    if (data.showLabels) {
      ctx.fillStyle = theme.metricLabel
      ctx.fillText(metric.labels.join(' · '), LABEL_INSET, metric.screenY - 2)
    }
  }

  // The origin and the advance bound the glyph's horizontal space; the
  // sidebearing lines sit inside them and are labelled one row lower, because
  // at a tight advance the two pairs would otherwise overprint.
  const verticals: {
    x: number
    label: string
    dashed: boolean
    row: 0 | 1
  }[] = [
    { x: 0, label: labels.origin, dashed: false, row: 0 },
    { x: metrics.width, label: labels.advance, dashed: false, row: 0 },
  ]
  if (metrics.lsb !== null)
    verticals.push({ x: metrics.lsb, label: labels.lsb, dashed: true, row: 1 })
  if (metrics.rsb !== null)
    verticals.push({
      x: metrics.width - metrics.rsb,
      label: labels.rsb,
      dashed: true,
      row: 1,
    })

  ctx.textBaseline = 'top'
  for (const vertical of verticals) {
    const x = crisp(viewport.toScreen({ x: vertical.x, y: 0 }).x)
    if (x < -1 || x > viewport.width + 1) continue
    ctx.strokeStyle = vertical.dashed ? theme.sidebearing : theme.metricStrong
    ctx.setLineDash(vertical.dashed ? [3, 3] : [])
    line(ctx, x, 0, x, viewport.height)
    ctx.setLineDash([])
  }

  if (data.showLabels) {
    ctx.fillStyle = theme.metricLabel
    ctx.textAlign = 'center'
    const baselineY = viewport.toScreen({ x: 0, y: 0 }).y
    const top = Math.min(viewport.height - 26, Math.max(2, baselineY + 4))
    for (const vertical of verticals) {
      const x = viewport.toScreen({ x: vertical.x, y: 0 }).x
      if (x < RULER_LABEL_MARGIN || x > viewport.width) continue
      ctx.fillText(vertical.label, x, top + vertical.row * 12)
    }
  }
  ctx.restore()
}
