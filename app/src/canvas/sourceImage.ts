import type { Viewport } from './Viewport'

/** A decoded source image and its placement in font units. */
export interface SourceImageDraw {
  bitmap: HTMLImageElement
  width: number
  height: number
  transform: [number, number, number, number, number, number]
}

/** Paints a translucent PNG behind the outline on a device-scaled context. */
export function drawSourceImage(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  image: SourceImageDraw,
): void {
  ctx.save()
  ctx.translate(viewport.offset.x, viewport.offset.y)
  ctx.scale(viewport.scale, -viewport.scale)
  ctx.transform(...image.transform)
  // Raster rows run down from the top; image placement uses a y-up origin.
  ctx.translate(0, image.height)
  ctx.scale(1, -1)
  ctx.globalAlpha *= 0.5
  ctx.drawImage(image.bitmap, 0, 0, image.width, image.height)
  ctx.restore()
}
