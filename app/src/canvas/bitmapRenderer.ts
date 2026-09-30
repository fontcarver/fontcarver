import type { Bitmap } from '../api'
import { sample, type PixelSelection } from '../views/bitmap/pixels'
import type { BitmapFrame } from './bitmapGeometry'

/** Extra editor guides; omitted for the compact waterfall renderer. */
export interface BitmapRenderOptions {
  frame?: BitmapFrame
  grid?: boolean
  advance?: number
  outlinePath?: string
  unitsPerPixel?: number
  resolved?: Bitmap
}

/** Paint pixels in document order; CSS colors are resolved by the caller. */
export function renderBitmap(
  canvas: HTMLCanvasElement,
  b: Bitmap,
  zoom: number,
  colors: { paper: string; ink: string; grid: string; accent: string },
  selection?: PixelSelection,
  guides = true,
  options: BitmapRenderOptions = {},
): void {
  const frame = options.frame ?? {
    left: b.origin_x,
    top: b.origin_y + b.height,
    width: b.width,
    height: b.height,
  }
  canvas.width = Math.max(1, Math.ceil(frame.width * zoom))
  canvas.height = Math.max(1, Math.ceil(frame.height * zoom))
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  if (guides) {
    ctx.fillStyle = colors.paper
    ctx.fillRect(0, 0, canvas.width, canvas.height)
  }
  const pixels = options.resolved ?? b
  const offsetX = (pixels.origin_x - frame.left) * zoom
  const offsetY = (frame.top - pixels.origin_y - pixels.height) * zoom
  for (let y = 0; y < pixels.height; y++)
    for (let x = 0; x < pixels.width; x++) {
      const value = sample(pixels, x, y)
      if (value === 0) continue
      if (pixels.depth === 32) {
        ctx.fillStyle = `rgba(${value >>> 24},${(value >>> 16) & 255},${(value >>> 8) & 255},${(value & 255) / 255})`
        ctx.globalAlpha = 1
      } else {
        ctx.fillStyle = colors.ink
        ctx.globalAlpha = value / ((1 << pixels.depth) - 1)
      }
      ctx.fillRect(offsetX + x * zoom, offsetY + y * zoom, zoom, zoom)
    }
  ctx.globalAlpha = 1
  if (guides && options.grid !== false && zoom >= 6) {
    ctx.strokeStyle = colors.grid
    ctx.lineWidth = 1
    ctx.beginPath()
    for (let x = 0; x <= frame.width; x++) {
      ctx.moveTo(x * zoom + 0.5, 0)
      ctx.lineTo(x * zoom + 0.5, canvas.height)
    }
    for (let y = 0; y <= frame.height; y++) {
      ctx.moveTo(0, y * zoom + 0.5)
      ctx.lineTo(canvas.width, y * zoom + 0.5)
    }
    ctx.stroke()
  }
  if (!guides) return
  ctx.strokeStyle = colors.accent
  ctx.lineWidth = 2
  const baseline = frame.top * zoom
  ctx.beginPath()
  ctx.moveTo(0, baseline)
  ctx.lineTo(canvas.width, baseline)
  const origin = -frame.left * zoom
  ctx.moveTo(origin, 0)
  ctx.lineTo(origin, canvas.height)
  ctx.stroke()
  if (options.advance !== undefined) {
    const advance = (options.advance - frame.left) * zoom
    ctx.setLineDash([6, 3])
    ctx.beginPath()
    ctx.moveTo(advance, 0)
    ctx.lineTo(advance, canvas.height)
    ctx.stroke()
    ctx.setLineDash([])
  }
  if (
    options.outlinePath &&
    options.unitsPerPixel &&
    typeof Path2D !== 'undefined'
  ) {
    ctx.save()
    const scale = zoom / options.unitsPerPixel
    ctx.translate(-frame.left * zoom, frame.top * zoom)
    ctx.scale(scale, -scale)
    ctx.lineWidth = 1.5 / scale
    ctx.stroke(new Path2D(options.outlinePath))
    ctx.restore()
  }
  if (selection) {
    ctx.setLineDash([4, 3])
    ctx.strokeRect(
      (b.origin_x - frame.left + selection.x) * zoom,
      (frame.top - b.origin_y - b.height + selection.y) * zoom,
      selection.width * zoom,
      selection.height * zoom,
    )
    ctx.setLineDash([])
  }
}
