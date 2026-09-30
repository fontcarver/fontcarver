import type { Viewport } from '../canvas/Viewport'
import type { EditorSource } from '../editor/editorTypes'
import type { GlyphScene } from '../views/glyph/glyphScene'
import type { GlyphTheme } from '../views/glyph/glyphTheme'
import type { PluginOverlay } from './contracts'
import { useFilterPreview } from './filterSession'
import { usePluginStore } from './pluginStore'
import { bundledPlugin, bundledPlugins } from './registry'

// Pointer movement repaints repeatedly, while preview path strings rarely change.
// Bound retained native paths even when users preview many glyphs in one session.
const paths = new Map<string, Path2D>()
const PATH_CACHE_LIMIT = 64

function parsedPath(data: string): Path2D {
  const existing = paths.get(data)
  if (existing) return existing
  const path = new Path2D(data)
  if (paths.size >= PATH_CACHE_LIMIT) {
    const oldest = paths.keys().next().value
    if (oldest !== undefined) paths.delete(oldest)
  }
  paths.set(data, path)
  return path
}

function drawOverlays(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  overlays: readonly PluginOverlay[],
  color: string,
  dashed: boolean,
): void {
  if (overlays.length === 0) return
  ctx.save()
  try {
    // renderGlyph leaves the device-pixel-ratio transform active. These are
    // CSS-pixel translations followed by the shared y-up font transform.
    ctx.translate(viewport.offset.x, viewport.offset.y)
    ctx.scale(viewport.scale, -viewport.scale)
    ctx.strokeStyle = color
    ctx.globalAlpha = 1
    ctx.lineWidth = 1.5 / viewport.scale
    ctx.setLineDash(dashed ? [6 / viewport.scale, 4 / viewport.scale] : [])
    for (const overlay of overlays) {
      if (overlay.kind === 'bounds') {
        const { x0, y0, x1, y1 } = overlay.bounds
        ctx.strokeRect(x0, y0, x1 - x0, y1 - y0)
      } else {
        ctx.stroke(parsedPath(overlay.path))
      }
    }
  } finally {
    ctx.restore()
  }
}

/** Paint enabled bundled annotations and the preview belonging to this editor. */
export function drawPluginOverlays(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  input: { source: EditorSource | null; scene: GlyphScene; theme: GlyphTheme },
): void {
  const { source, scene, theme } = input
  if (!source) return
  const { installed } = usePluginStore.getState()
  const bbox = source.outline.bbox
  // Null is authoritative: an empty selected layer must not inherit bounds
  // belonging to the default master represented by an older scene response.
  const bounds =
    bbox === undefined
      ? scene.bbox
      : bbox
        ? { x0: bbox[0], y0: bbox[1], x1: bbox[2], y1: bbox[3] }
        : null
  for (const plugin of bundledPlugins) {
    if ('report' in plugin && installed[plugin.manifest.id]?.enabled)
      drawOverlays(
        ctx,
        viewport,
        plugin.report({ bounds }),
        theme.accent,
        false,
      )
  }

  const preview = useFilterPreview.getState().preview
  if (!preview || !installed[preview.pluginId]?.enabled) return
  const plugin = bundledPlugin(preview.pluginId)
  if (!plugin || !('plan' in plugin)) return
  const { target } = preview
  if (
    target.doc !== source.doc ||
    target.gid !== source.gid ||
    target.glyph !== source.glyph ||
    target.layer !== source.outline.layer ||
    target.master !== source.master
  )
    return
  drawOverlays(ctx, viewport, preview.overlays, theme.warning, true)
}
