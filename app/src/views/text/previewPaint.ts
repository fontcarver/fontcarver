import type { PreviewGlyph } from './presentationTypes'

export const PREVIEW_SIZES = [12, 24, 48] as const
export const PREVIEW_CANVAS_HEIGHT = 70
export const PREVIEW_CANVAS_MIN_WIDTH = 160
export const PREVIEW_CANVAS_MAX_WIDTH = 4_096

const PREVIEW_PADDING = 8

export interface PreviewGeometry {
  cssWidth: number
  minX: number
  cssHeight?: number
  maxY?: number
}

/** Bounds a horizontal run while retaining a useful scrollable preview width. */
export function previewGeometry(
  glyphs: readonly PreviewGlyph[],
  size: number,
  unitsPerEm: number,
): PreviewGeometry {
  let minX = 0
  let maxX = 0
  for (const glyph of glyphs) {
    minX = Math.min(minX, glyph.x, glyph.x + glyph.xAdvance)
    maxX = Math.max(maxX, glyph.x, glyph.x + glyph.xAdvance)
  }
  const scale = size / unitsPerEm
  const vertical = glyphs.some((g) => g.yAdvance !== 0)
  const minY = Math.min(
    0,
    ...glyphs.map((g) => g.y + Math.min(0, g.yAdvance) - unitsPerEm * 0.2),
  )
  const maxY = Math.max(0, ...glyphs.map((g) => g.y + unitsPerEm))
  if (vertical) maxX = Math.max(maxX, minX + unitsPerEm)
  const contentWidth = Math.ceil((maxX - minX) * scale + PREVIEW_PADDING * 2)
  return {
    cssWidth: Math.min(
      PREVIEW_CANVAS_MAX_WIDTH,
      Math.max(PREVIEW_CANVAS_MIN_WIDTH, contentWidth),
    ),
    minX,
    ...(size > 48
      ? {
          cssHeight: Math.min(
            PREVIEW_CANVAS_MAX_WIDTH,
            Math.ceil(size * 1.2 + PREVIEW_PADDING * 2),
          ),
        }
      : {}),
    ...(vertical
      ? {
          cssHeight: Math.min(
            PREVIEW_CANVAS_MAX_WIDTH,
            Math.max(
              PREVIEW_CANVAS_HEIGHT,
              Math.ceil((maxY - minY) * scale + PREVIEW_PADDING * 2),
            ),
          ),
          maxY,
        }
      : {}),
  }
}

export interface PaintPreviewOptions {
  size: number
  unitsPerEm: number
  width: number
  height: number
  pixelRatio: number
  minX: number
  maxY?: number
  ink: string
  flipHorizontal?: boolean
  flipVertical?: boolean
  blur?: number
}

/** Paints supplied paths with their shaped offsets using a y-up font transform. */
export function paintPreview(
  context: CanvasRenderingContext2D,
  glyphs: readonly PreviewGlyph[],
  options: PaintPreviewOptions,
): void {
  const { size, unitsPerEm, width, height, pixelRatio, minX, ink } = options
  context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0)
  context.filter = 'none'
  context.clearRect(0, 0, width, height)
  context.fillStyle = ink
  const blur = Number.isFinite(options.blur)
    ? Math.max(0, Math.min(4, options.blur ?? 0))
    : 0
  context.filter = blur > 0 ? `blur(${blur * pixelRatio}px)` : 'none'

  const scale = size / unitsPerEm
  const baseline =
    options.maxY !== undefined
      ? PREVIEW_PADDING + options.maxY * scale
      : Math.min(height - PREVIEW_PADDING, Math.round((height + size) / 2))
  const offsetX = PREVIEW_PADDING - minX * scale
  context.setTransform(
    (options.flipHorizontal ? -scale : scale) * pixelRatio,
    0,
    0,
    (options.flipVertical ? scale : -scale) * pixelRatio,
    (options.flipHorizontal ? width - offsetX : offsetX) * pixelRatio,
    (options.flipVertical ? height - baseline : baseline) * pixelRatio,
  )

  for (const glyph of glyphs) {
    if (glyph.color) {
      context.save()
      context.translate(glyph.x, glyph.y)
      glyph.color.paint(context, ink)
      context.restore()
      continue
    }
    if (!glyph.path) continue
    let path: Path2D
    try {
      path = new Path2D(glyph.path)
    } catch {
      continue
    }
    context.save()
    context.translate(glyph.x, glyph.y)
    context.fill(path)
    context.restore()
  }
}
