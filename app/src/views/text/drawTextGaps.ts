import type { Viewport } from '../../canvas/Viewport'
import type { GlyphTheme } from '../glyph/glyphTheme'
import type { TextCanvasRun } from './textCanvas'
import { appliedKerning } from './textGaps'

/** Font-unit height of the gap handles above the baseline, as a fraction of em. */
const HANDLE_DROP = -0.12

/**
 * Ink for a kerning value: tighter is the warning colour, looser is the
 * smooth-node hue, exactly zero is the accent, and an unknown value keeps
 * `fallback` — the accent for the boundary being edited, neutral otherwise.
 */
function kerningInk(
  theme: GlyphTheme,
  value: number | null | undefined,
  fallback: string,
): string {
  if (value === null || value === undefined || !Number.isFinite(value))
    return fallback
  if (value < 0) return theme.warning
  if (value > 0) return theme.nodeSmooth
  return theme.accent
}

function label(value: number): string {
  return String(Math.round(value * 10) / 10)
}

/** The stored advance of one run glyph in the shown master, if it is loaded. */
function storedAdvance(
  run: TextCanvasRun,
  index: number,
  vertical: boolean,
): number | undefined {
  const glyph = run.glyphs[index]
  if (!glyph || glyph.documentGid === null) return undefined
  const cell = run.cells.get(glyph.documentGid)
  if (!cell) return undefined
  return vertical ? (cell.vertical_advance ?? undefined) : cell.width
}

/** Draws a small pill of text at a screen point, centred horizontally. */
function pill(
  ctx: CanvasRenderingContext2D,
  theme: GlyphTheme,
  text: string,
  x: number,
  y: number,
  ink: string,
): void {
  ctx.font = `10px ${theme.fontFamily}`
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  const width = ctx.measureText(text).width + 6
  ctx.globalAlpha = 0.85
  ctx.fillStyle = theme.hudBackground
  ctx.fillRect(x - width / 2, y - 7, width, 14)
  ctx.globalAlpha = 1
  ctx.fillStyle = ink
  ctx.fillText(text, x, y)
}

/**
 * Hides everything outside the cut-off band.
 *
 * RoboFont's Cut Off display keeps only a horizontal slice of the run on
 * screen, which is how counters and sidebearings become comparable without the
 * rest of the letter competing for attention. The glyphs are already painted at
 * this point, so the band is realised by covering what is outside it with the
 * canvas background rather than by clipping the earlier pass.
 */
function drawCutOff(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  run: TextCanvasRun,
  theme: GlyphTheme,
): void {
  const band = run.cutOff
  if (!band) return
  const from = Math.min(band.from, band.to)
  const to = Math.max(band.from, band.to)
  const top = viewport.toScreen({ x: 0, y: to - run.activeOrigin.y }).y
  const bottom = viewport.toScreen({ x: 0, y: from - run.activeOrigin.y }).y
  const width = viewport.width
  ctx.fillStyle = theme.background
  if (top > 0) ctx.fillRect(0, 0, width, top)
  if (bottom < viewport.height)
    ctx.fillRect(0, bottom, width, viewport.height - bottom)
  ctx.strokeStyle = theme.metric
  ctx.lineWidth = 1
  ctx.setLineDash([2, 4])
  ctx.beginPath()
  for (const y of [top, bottom]) {
    ctx.moveTo(0, y)
    ctx.lineTo(width, y)
  }
  ctx.stroke()
  ctx.setLineDash([])
}

/**
 * Draws the measurement beam and the margins measured on it.
 *
 * The beam answers "how much space is there at this height", which outline
 * bounds cannot: a `V` and an `H` with identical bounds have completely
 * different margins at the x-height. The kernel measures each glyph's margins
 * at the beam height; the distance printed at each boundary is the sum of the
 * two facing margins plus the kerning the run applied, which is the real
 * distance between the two outlines on that line.
 */
function drawBeam(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  run: TextCanvasRun,
  theme: GlyphTheme,
  visible: readonly number[],
): void {
  const beam = run.beam
  if (!beam) return
  const y = viewport.toScreen({ x: 0, y: beam.height - run.activeOrigin.y }).y
  ctx.strokeStyle = theme.accent
  ctx.lineWidth = 1
  ctx.setLineDash([6, 3])
  ctx.beginPath()
  ctx.moveTo(0, y)
  ctx.lineTo(viewport.width, y)
  ctx.stroke()
  ctx.setLineDash([])

  const vertical = run.gaps?.some((gap) => gap.y !== undefined) ?? false
  if (vertical) return
  for (const index of visible) {
    const left = run.glyphs[index]
    const right = run.glyphs[index + 1]
    if (!left?.name || !right?.name) continue
    const leftMargins = beam.margins.get(left.name)
    const rightMargins = beam.margins.get(right.name)
    if (!leftMargins || !rightMargins) continue
    const advance = storedAdvance(run, index, false)
    const kerning = appliedKerning(left, advance, false) ?? 0
    const distance = leftMargins.right + rightMargins.left + kerning
    const inkEnd = left.x + (advance ?? left.xAdvance) - leftMargins.right
    const inkStart = right.x + rightMargins.left
    const start = viewport.toScreen({
      x: inkEnd - run.activeOrigin.x,
      y: beam.height - run.activeOrigin.y,
    })
    const end = viewport.toScreen({
      x: inkStart - run.activeOrigin.x,
      y: beam.height - run.activeOrigin.y,
    })
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 3
    ctx.beginPath()
    ctx.moveTo(start.x, start.y)
    ctx.lineTo(end.x, end.y)
    ctx.stroke()
    ctx.lineWidth = 1
    pill(
      ctx,
      theme,
      label(distance),
      (start.x + end.x) / 2,
      start.y - 12,
      theme.metricLabel,
    )
  }
}

/** Draws every editable boundary handle, coloured by the kerning applied there. */
function drawGapHandles(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  run: TextCanvasRun,
  theme: GlyphTheme,
  visible: readonly number[],
): void {
  if (!run.gaps) return
  const shown = new Set(visible)
  for (const gap of run.gaps) {
    if (
      gap.reason !== null ||
      (!shown.has(gap.index) && !shown.has(gap.index + 1))
    )
      continue
    const vertical = gap.y !== undefined
    const point = viewport.toScreen({
      x: gap.x - (vertical ? run.unitsPerEm / 2 : 0) - run.activeOrigin.x,
      y: (gap.y ?? run.unitsPerEm * HANDLE_DROP) - run.activeOrigin.y,
    })
    const selected = run.selectedGap === gap.index
    // The selected boundary reports the resolved stored value, which is the
    // number being edited; every other boundary reports what the shaped run
    // actually applied, so the whole line shows where its kerning is.
    const value = selected
      ? (run.selectedKerningValue ?? null)
      : appliedKerning(
          run.glyphs[gap.index],
          storedAdvance(run, gap.index, vertical),
          vertical,
        )
    const ink = kerningInk(
      theme,
      value,
      selected ? theme.accent : theme.metricStrong,
    )
    ctx.fillStyle =
      selected || (value !== null && value !== 0) ? ink : theme.metricStrong
    if (selected) {
      const top = viewport.toScreen({
        x: gap.x + (vertical ? run.unitsPerEm / 2 : 0) - run.activeOrigin.x,
        y: (gap.y ?? run.unitsPerEm * 0.8) - run.activeOrigin.y,
      })
      ctx.strokeStyle = ink
      ctx.lineWidth = 1
      ctx.setLineDash([3, 3])
      ctx.beginPath()
      ctx.moveTo(point.x, point.y)
      ctx.lineTo(top.x, top.y)
      ctx.stroke()
      ctx.setLineDash([])
    }
    const radius = selected ? 5 : 3
    ctx.beginPath()
    ctx.moveTo(point.x, point.y - radius)
    ctx.lineTo(point.x + radius, point.y)
    ctx.lineTo(point.x, point.y + radius)
    ctx.lineTo(point.x - radius, point.y)
    ctx.closePath()
    ctx.fill()
    // An exception departs from its class rule, so it gets a ring: colour is
    // never the only signal, and the number is spelled out beside it. Every
    // exception is ringed, not only the selected one, so a run shows where its
    // class rules have been overridden.
    const exception = selected
      ? (run.selectedKerningException ?? run.kerningExceptions?.has(gap.index))
      : run.kerningExceptions?.has(gap.index)
    if (exception) {
      ctx.strokeStyle = ink
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.arc(point.x, point.y, radius + 3, 0, Math.PI * 2)
      ctx.stroke()
    }
    if (value !== null && value !== 0)
      pill(
        ctx,
        theme,
        label(value),
        point.x,
        point.y + radius + 9,
        selected ? ink : theme.metricLabel,
      )
  }
}

/** Draws gap handles, the spacing beam and the cut-off band in screen space. */
export function drawTextGaps(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  run: TextCanvasRun,
  theme: GlyphTheme,
  visible: readonly number[],
): void {
  if (run.mode !== 'text') return
  ctx.save()
  ctx.setTransform(viewport.dpr, 0, 0, viewport.dpr, 0, 0)
  drawCutOff(ctx, viewport, run, theme)
  drawBeam(ctx, viewport, run, theme, visible)
  drawGapHandles(ctx, viewport, run, theme, visible)
  ctx.restore()
}
