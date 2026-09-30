import type { FontBox, Viewport } from '../canvas/Viewport'
import type { EditorSource } from '../editor/editorTypes'
import type { GlyphScene } from '../views/glyph/glyphScene'
import type { GlyphTheme } from '../views/glyph/glyphTheme'
import type { PluginOverlay, ReporterViewContext } from './contracts'
import { useFilterPreview } from './filterSession'
import { pluginSettings, usePluginStore } from './pluginStore'
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

// A reporter that throws is reported once per session: repainting runs on every
// pointer move, and a console flooded with the same failure helps nobody.
const failed = new Set<string>()

function reportFailure(id: string, error: unknown): void {
  if (failed.has(id)) return
  failed.add(id)
  console.error(
    `FontCarver plugin ${id} failed while reporting overlays`,
    error,
  )
}

/** Forgets reported failures, so a reinstalled plugin reports again. */
export function resetPluginFailures(): void {
  failed.clear()
}

/** The visible font-space rectangle and zoom a reporter draws against. */
export function viewContext(viewport: Viewport): ReporterViewContext {
  const topLeft = viewport.toFont({ x: 0, y: 0 })
  const bottomRight = viewport.toFont({
    x: viewport.width,
    y: viewport.height,
  })
  const visible: FontBox = {
    x0: Math.min(topLeft.x, bottomRight.x),
    y0: Math.min(topLeft.y, bottomRight.y),
    x1: Math.max(topLeft.x, bottomRight.x),
    y1: Math.max(topLeft.y, bottomRight.y),
  }
  return { scale: viewport.scale, visible }
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
      } else if (overlay.kind === 'line') {
        ctx.beginPath()
        ctx.moveTo(overlay.from[0], overlay.from[1])
        ctx.lineTo(overlay.to[0], overlay.to[1])
        ctx.stroke()
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
  const document = {
    doc: source.doc,
    gid: source.gid,
    glyph: source.glyph,
    layer: source.outline.layer,
    master: source.master,
    metrics: source.metrics,
  }
  const view = viewContext(viewport)
  for (const plugin of bundledPlugins) {
    if (!('report' in plugin) || !installed[plugin.manifest.id]?.enabled)
      continue
    // A failing reporter loses its own drawing, never the canvas paint.
    let overlays: readonly PluginOverlay[]
    try {
      overlays = plugin.report({
        bounds,
        document,
        view,
        settings: pluginSettings(plugin.manifest.id),
      })
    } catch (error) {
      reportFailure(plugin.manifest.id, error)
      continue
    }
    drawOverlays(ctx, viewport, overlays, theme.accent, false)
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
