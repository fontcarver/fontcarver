import type { ColorPreviewLayer } from '../api'

/** A ready color glyph painted at its origin in y-up font coordinates. */
export interface ColorGlyphPaint {
  paint(context: CanvasRenderingContext2D, foreground: string): void
}

/** A color painter with owned browser resources released by its cache. */
export interface ColorGlyphResource extends ColorGlyphPaint {
  dispose(): void
}

/** Parse validated COLRv0 paths once and retain palette order and alpha. */
export function colorLayers(
  layers: readonly ColorPreviewLayer[],
): ColorGlyphResource {
  const paths = layers.map((layer) => ({
    path: new Path2D(layer.path),
    color: layer.color,
  }))
  return {
    paint(context, foreground) {
      context.save()
      for (const { path, color } of paths) {
        context.fillStyle = color
          ? `rgb(${color.red} ${color.green} ${color.blue})`
          : foreground
        context.globalAlpha = color ? color.alpha / 255 : 1
        context.fill(path, 'nonzero')
      }
      context.restore()
    },
    dispose() {},
  }
}
