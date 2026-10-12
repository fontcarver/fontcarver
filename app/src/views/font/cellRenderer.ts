/**
 * Painting of one glyph-grid cell.
 *
 * Plain TypeScript: the React component owns the `<canvas>` element and its
 * lifecycle and calls {@link paintCell} with an already-resolved theme. Sizes
 * are CSS pixels; the device pixel ratio is applied here so callers never have
 * to think in device pixels.
 */

import {
  emBoxFor,
  fitGlyph,
  paintBaseline,
  paintEmptyMarker,
  paintGlyph,
  paintHatch,
  type Rect,
} from '../../canvas/glyphPaint'
import type { GlyphCell } from '../../api'
import type { ColorGlyphPaint } from '../../canvas/colorGlyph'
import type { LabelMode } from '../../stores/documentStore'

import { colorLabelSwatch } from './colorLabels'

/** Colours a cell paints with, resolved from the design tokens by the grid. */
export interface CellTheme {
  background: string
  backgroundSelected: string
  glyph: string
  glyphMuted: string
  label: string
  ring: string
  focusRing: string
  baseline: string
  badge: string
  border: string
}

/** Everything {@link paintCell} needs besides the cell itself. */
export interface PaintCellOptions {
  /** Cell edge length in CSS pixels. */
  size: number
  /** Device pixel ratio the backing store was sized for. */
  dpr: number
  /** Font units per em. */
  upem: number
  /** Typographic ascender in font units. */
  ascender: number
  /** Typographic descender in font units. */
  descender: number
  /** What the label under the glyph shows. */
  labelMode: LabelMode
  /** Position of the cell in the current filtered, sorted order. */
  index: number
  selected: boolean
  focused: boolean
  theme: CellTheme
  /** Cached outline, or `null` for an empty glyph. */
  path: Path2D | null
  /** Validated color artwork takes priority, including outline-free glyphs. */
  color?: ColorGlyphPaint
  /** UI font stack used for the label. */
  fontFamily: string
  /**
   * Whether an empty cell may show its character drawn with a system face.
   *
   * Presentation only: nothing is written to the glyph, and the character is
   * drawn in the muted colour so it never reads as the font's own outline.
   */
  referenceCharacters?: boolean
}

/**
 * Smallest cell that still has room for a readable label. Below this the name
 * is only available as the cell's tooltip.
 */
export const LABEL_MIN_SIZE = 32

/** Cell size at which the label switches to the full-size type. */
const LABEL_LARGE_SIZE = 48

/** Height of the colour-label stripe along the top of a cell. */
const STRIPE_HEIGHT = 3

/** Formats one Unicode scalar the way the industry writes it. */
export function formatUnicode(unicode: number): string {
  return `U+${unicode.toString(16).toUpperCase().padStart(4, '0')}`
}

/** The text drawn under the glyph for one cell. */
export function cellLabel(
  cell: GlyphCell,
  mode: LabelMode,
  index: number,
): string {
  if (mode === 'index') return String(index)
  if (mode === 'unicode') {
    const first = cell.unicodes[0]
    return first === undefined ? '—' : formatUnicode(first)
  }
  return cell.name
}

function ellipsise(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
): string {
  if (ctx.measureText(text).width <= maxWidth) return text
  let cut = text.length - 1
  while (cut > 0 && ctx.measureText(`${text.slice(0, cut)}…`).width > maxWidth)
    cut -= 1
  return cut <= 0 ? '' : `${text.slice(0, cut)}…`
}

/** A reference character is presentation only, never synthetic glyph content. */
function emptyGlyphCharacter(cell: GlyphCell): string | undefined {
  if (!cell.is_empty) return undefined
  for (const scalar of cell.unicodes) {
    if (
      !Number.isInteger(scalar) ||
      scalar < 0 ||
      scalar > 0x10ffff ||
      (scalar >= 0xd800 && scalar <= 0xdfff)
    )
      continue
    const character = String.fromCodePoint(scalar)
    if (
      !/[\p{Cc}\p{Cf}\p{Z}\p{Default_Ignorable_Code_Point}]/u.test(character)
    ) {
      return character
    }
  }
  return undefined
}

function paintReferenceCharacter(
  ctx: CanvasRenderingContext2D,
  character: string,
  box: Rect,
  options: PaintCellOptions,
) {
  ctx.save()
  ctx.beginPath()
  ctx.rect(box.x, box.y, box.width, box.height)
  ctx.clip()
  ctx.font = `${Math.max(1, Math.round(box.height * 0.8))}px ${options.fontFamily}`
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillStyle = options.theme.glyphMuted
  ctx.fillText(
    character,
    box.x + box.width / 2,
    box.y + box.height / 2,
    box.width,
  )
  ctx.restore()
}

/**
 * Paints one cell onto a canvas context whose backing store is
 * `size * dpr` square. The context is reset to the identity transform on entry
 * and left scaled by `dpr` on exit, so the caller may keep re-using it.
 */
export function paintCell(
  ctx: CanvasRenderingContext2D,
  cell: GlyphCell,
  options: PaintCellOptions,
): void {
  const { size, dpr, theme } = options
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, size, size)

  ctx.fillStyle = options.selected ? theme.backgroundSelected : theme.background
  ctx.fillRect(0, 0, size, size)

  const showLabel = size >= LABEL_MIN_SIZE
  const labelHeight = showLabel ? (size >= LABEL_LARGE_SIZE ? 14 : 10) : 0
  const pad = Math.max(2, Math.round(size * 0.08))
  const box: Rect = {
    x: pad,
    y: pad + STRIPE_HEIGHT,
    width: Math.max(1, size - pad * 2),
    height: Math.max(1, size - pad * 2 - STRIPE_HEIGHT - labelHeight),
  }

  if (!cell.export)
    paintHatch(ctx, { x: 0, y: 0, width: size, height: size }, theme.glyphMuted)

  const swatch = colorLabelSwatch(cell.color_label)
  if (swatch) {
    ctx.fillStyle = swatch
    ctx.fillRect(0, 0, size, STRIPE_HEIGHT)
  }

  const em = emBoxFor(options.upem, options.ascender, options.descender)
  const fit = fitGlyph(box, cell.width, em)
  paintBaseline(ctx, box, fit, theme.baseline)

  if (options.color) {
    ctx.save()
    ctx.beginPath()
    ctx.rect(box.x, box.y, box.width, box.height)
    ctx.clip()
    ctx.translate(fit.originX, fit.baselineY)
    ctx.scale(fit.scale, -fit.scale)
    options.color.paint(ctx, theme.glyph)
    ctx.restore()
  } else if (cell.is_empty || options.path === null) {
    const character =
      options.referenceCharacters === false
        ? undefined
        : emptyGlyphCharacter(cell)
    if (character) paintReferenceCharacter(ctx, character, box, options)
    else paintEmptyMarker(ctx, box, theme.glyphMuted)
  } else {
    paintGlyph(ctx, options.path, fit, theme.glyph)
  }

  if (cell.has_components) {
    const dot = Math.max(3, Math.round(size * 0.07))
    ctx.fillStyle = theme.badge
    ctx.fillRect(size - pad - dot, pad + STRIPE_HEIGHT, dot, dot)
  }

  // The layer's own label is independent of the glyph's, so it gets its own
  // mark rather than sharing the stripe: a glyph can carry both at once.
  const layerSwatch = colorLabelSwatch(cell.layer_color_label ?? null)
  if (layerSwatch) {
    const dot = Math.max(3, Math.round(size * 0.07))
    ctx.fillStyle = layerSwatch
    ctx.beginPath()
    ctx.moveTo(pad, pad + STRIPE_HEIGHT)
    ctx.lineTo(pad + dot, pad + STRIPE_HEIGHT)
    ctx.lineTo(pad, pad + STRIPE_HEIGHT + dot)
    ctx.closePath()
    ctx.fill()
  }

  if (labelHeight > 0) {
    const fontSize =
      size >= LABEL_LARGE_SIZE
        ? Math.min(11, Math.max(9, Math.round(size * 0.16)))
        : 8
    ctx.font = `${fontSize}px ${options.fontFamily}`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'alphabetic'
    ctx.fillStyle = theme.label
    const text = ellipsise(
      ctx,
      cellLabel(cell, options.labelMode, options.index),
      size - pad,
    )
    ctx.fillText(text, size / 2, size - Math.max(1, pad - 1))
  }

  if (options.selected || options.focused) {
    ctx.strokeStyle = options.focused ? theme.focusRing : theme.ring
    ctx.lineWidth = options.focused ? 2 : 1.5
    const inset = options.focused ? 1 : 0.75
    ctx.strokeRect(inset, inset, size - inset * 2, size - inset * 2)
  } else {
    ctx.strokeStyle = theme.border
    ctx.lineWidth = 1
    ctx.strokeRect(0.5, 0.5, size - 1, size - 1)
  }
}
