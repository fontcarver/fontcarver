/**
 * Glyph painting primitives.
 *
 * Font space is y-up with the baseline at y = 0; canvas space is y-down. Every
 * function here works in CSS pixels on an already device-pixel-scaled context
 * and never touches React.
 */

/** Vertical reference frame a grid cell fits its glyph into. */
export interface EmBox {
  /** Font units per em. */
  upem: number
  /** Typographic ascender in font units. */
  ascender: number
  /** Typographic descender in font units, negative below the baseline. */
  descender: number
}

/** An axis-aligned rectangle in CSS pixels. */
export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

/** Where and how large a glyph is drawn inside its cell. */
export interface GlyphFit {
  /** Font units to CSS pixels. */
  scale: number
  /** Canvas x of the glyph origin. */
  originX: number
  /** Canvas y of the baseline. */
  baselineY: number
}

/**
 * Builds an em box, falling back to the conventional 80 % / 20 % split of the
 * em when a font reports no usable vertical metrics.
 */
export function emBoxFor(
  upem: number,
  ascender?: number | null,
  descender?: number | null,
): EmBox {
  const em = upem > 0 ? upem : 1000
  const top = ascender != null && ascender > 0 ? ascender : em * 0.8
  const bottom = descender != null && descender < 0 ? descender : -em * 0.2
  return { upem: em, ascender: top, descender: bottom }
}

/**
 * Fits one glyph's em box into `box`, uniformly scaled and centred. Every cell
 * of a font uses the same vertical scale so the glyphs line up on a common
 * baseline; only the horizontal centring depends on the advance width.
 */
export function fitGlyph(box: Rect, advance: number, em: EmBox): GlyphFit {
  const emHeight = Math.max(1, em.ascender - em.descender)
  const emWidth = advance > 0 ? advance : em.upem
  const scale = Math.min(
    box.width / Math.max(emWidth, 1),
    box.height / emHeight,
  )
  const originX = box.x + (box.width - emWidth * scale) / 2
  const baselineY =
    box.y + (box.height - emHeight * scale) / 2 + em.ascender * scale
  return { scale, originX, baselineY }
}

/** Fills one outline, mapping font space onto the fitted cell area. */
export function paintGlyph(
  ctx: CanvasRenderingContext2D,
  path: Path2D,
  fit: GlyphFit,
  fill: string,
): void {
  ctx.save()
  ctx.translate(fit.originX, fit.baselineY)
  ctx.scale(fit.scale, -fit.scale)
  ctx.fillStyle = fill
  ctx.fill(path, 'nonzero')
  ctx.restore()
}

/** Draws the dotted box that marks a glyph with no editable content. */
export function paintEmptyMarker(
  ctx: CanvasRenderingContext2D,
  box: Rect,
  stroke: string,
): void {
  const inset = Math.max(2, Math.round(Math.min(box.width, box.height) * 0.18))
  ctx.save()
  ctx.strokeStyle = stroke
  ctx.lineWidth = 1
  ctx.setLineDash([2, 2])
  ctx.strokeRect(
    box.x + inset + 0.5,
    box.y + inset + 0.5,
    Math.max(1, box.width - inset * 2 - 1),
    Math.max(1, box.height - inset * 2 - 1),
  )
  ctx.restore()
}

/** Draws the short baseline ticks at the left and right of the cell area. */
export function paintBaseline(
  ctx: CanvasRenderingContext2D,
  box: Rect,
  fit: GlyphFit,
  stroke: string,
): void {
  const y = Math.round(fit.baselineY) + 0.5
  if (y < box.y || y > box.y + box.height) return
  const tick = Math.max(3, Math.round(box.width * 0.12))
  ctx.save()
  ctx.strokeStyle = stroke
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(box.x, y)
  ctx.lineTo(box.x + tick, y)
  ctx.moveTo(box.x + box.width - tick, y)
  ctx.lineTo(box.x + box.width, y)
  ctx.stroke()
  ctx.restore()
}

/** Fills a rectangle with the diagonal hatch that marks a non-exporting glyph. */
export function paintHatch(
  ctx: CanvasRenderingContext2D,
  box: Rect,
  stroke: string,
): void {
  const step = 6
  ctx.save()
  ctx.beginPath()
  ctx.rect(box.x, box.y, box.width, box.height)
  ctx.clip()
  ctx.strokeStyle = stroke
  ctx.lineWidth = 1
  ctx.beginPath()
  for (let offset = -box.height; offset < box.width; offset += step) {
    ctx.moveTo(box.x + offset, box.y + box.height)
    ctx.lineTo(box.x + offset + box.height, box.y)
  }
  ctx.stroke()
  ctx.restore()
}
