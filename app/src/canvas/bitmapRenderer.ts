import type { Bitmap } from '../api'
import { sample, type PixelSelection } from '../views/bitmap/pixels'
/** Paint pixels in document order; CSS colors are resolved by the caller. */
export function renderBitmap(
  canvas: HTMLCanvasElement,
  b: Bitmap,
  zoom: number,
  colors: { paper: string; ink: string; grid: string; accent: string },
  selection?: PixelSelection,
  guides = true,
): void {
  canvas.width = Math.max(1, Math.ceil(b.width * zoom))
  canvas.height = Math.max(1, Math.ceil(b.height * zoom))
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  if (guides) {
    ctx.fillStyle = colors.paper
    ctx.fillRect(0, 0, canvas.width, canvas.height)
  }
  for (let y = 0; y < b.height; y++)
    for (let x = 0; x < b.width; x++) {
      const value = sample(b, x, y)
      if (value === 0) continue
      if (b.depth === 32) {
        ctx.fillStyle = `rgba(${value >>> 24},${(value >>> 16) & 255},${(value >>> 8) & 255},${(value & 255) / 255})`
        ctx.globalAlpha = 1
      } else {
        ctx.fillStyle = colors.ink
        ctx.globalAlpha = value / ((1 << b.depth) - 1)
      }
      ctx.fillRect(x * zoom, y * zoom, zoom, zoom)
    }
  ctx.globalAlpha = 1
  if (guides && zoom >= 6) {
    ctx.strokeStyle = colors.grid
    ctx.lineWidth = 1
    ctx.beginPath()
    for (let x = 0; x <= b.width; x++) {
      ctx.moveTo(x * zoom + 0.5, 0)
      ctx.lineTo(x * zoom + 0.5, canvas.height)
    }
    for (let y = 0; y <= b.height; y++) {
      ctx.moveTo(0, y * zoom + 0.5)
      ctx.lineTo(canvas.width, y * zoom + 0.5)
    }
    ctx.stroke()
  }
  if (!guides) return
  ctx.strokeStyle = colors.accent
  ctx.lineWidth = 2
  const baseline = (b.height + b.origin_y) * zoom
  ctx.beginPath()
  ctx.moveTo(0, baseline)
  ctx.lineTo(canvas.width, baseline)
  ctx.stroke()
  if (selection) {
    ctx.setLineDash([4, 3])
    ctx.strokeRect(
      selection.x * zoom,
      selection.y * zoom,
      selection.width * zoom,
      selection.height * zoom,
    )
  }
}
