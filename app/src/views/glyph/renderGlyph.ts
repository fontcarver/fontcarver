/**
 * Composes one frame of the glyph editor from the individual drawing layers.
 *
 * The order is deliberate: metrics and guides sit behind the glyph, components
 * behind the layer's own contours, and anchors and rulers in front of
 * everything. Keeping the composition in one plain function means the React
 * component only has to decide *when* to draw, never *what*.
 */

import type { Viewport } from '../../canvas/Viewport'

import { drawAnchors } from './drawAnchors'
import { IDENTITY } from '../../editor/affine'
import { drawComponents, type ComponentDraw } from './drawComponents'
import { drawEditing, NO_OVERLAY, type EditingOverlay } from './drawEditing'
import { drawGuides } from './drawGuides'
import { drawHud } from './drawHud'
import { drawMetrics, type MetricLabels } from './drawMetrics'
import { drawOutline } from './drawOutline'
import { drawGrid, drawToolOverlay } from './drawTools'
import { drawRulers } from './drawRulers'
import type { GlyphScene } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'
import { NO_TOOL_OVERLAY, type ToolOverlay } from '../../editor/tools/toolTypes'

/** Which overlays the editor is currently showing. */
export interface GlyphOverlays {
  /** Fill the outline instead of stroking it. */
  filled: boolean
  points: boolean
  metrics: boolean
  anchors: boolean
  guides: boolean
  rulers: boolean
}

/** Every overlay on, which is the default for a read-only glyph tab. */
export const DEFAULT_OVERLAYS: GlyphOverlays = {
  filled: false,
  points: true,
  metrics: true,
  anchors: true,
  guides: true,
  rulers: true,
}

/** Everything one frame needs. */
export interface RenderGlyphOptions {
  scene: GlyphScene
  theme: GlyphTheme
  labels: MetricLabels
  overlays: GlyphOverlays
  /** Component-flattened outline of the shown layer, in font units. */
  flattened: Path2D | null
  /**
   * The layer's components with their own paths and any preview offset. When
   * omitted, the scene's components draw as markers over the flattened path.
   */
  components?: readonly ComponentDraw[]
  /** Pointer position in font units, or `null` when the pointer is away. */
  pointer: { x: number; y: number } | null
  /** Selection keys, drawn as the selection layer of the outline. */
  selected?: ReadonlySet<string>
  /** Transient editing feedback: marquee, snap halo, ghosts, alignment lines. */
  editing?: EditingOverlay
  /** Cursor-HUD text, or `null` when no gesture is running. */
  hud?: string | null
  /** Pointer position in canvas pixels, where the HUD is anchored. */
  pointerScreen?: { x: number; y: number } | null
  /** Transient geometry the active drawing tool is showing. */
  tool?: ToolOverlay
  /** Grid step in font units; zero draws no grid. */
  grid?: number
}

/** Paints one frame of the glyph editor. */
export function renderGlyph(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  options: RenderGlyphOptions,
): void {
  const { scene, theme, overlays } = options
  ctx.setTransform(viewport.dpr, 0, 0, viewport.dpr, 0, 0)
  ctx.fillStyle = theme.background
  ctx.fillRect(0, 0, viewport.width, viewport.height)

  drawGrid(ctx, viewport, options.grid ?? 0, theme)

  if (overlays.metrics)
    drawMetrics(
      ctx,
      viewport,
      { metrics: scene.metrics, labels: options.labels, showLabels: true },
      theme,
    )

  if (overlays.guides)
    drawGuides(ctx, viewport, { guides: scene.guides, showLabels: true }, theme)

  drawComponents(
    ctx,
    viewport,
    {
      path: options.flattened,
      components:
        options.components ??
        scene.components.map((component) => ({
          component,
          path: null,
          transform: IDENTITY,
        })),
      filled: overlays.filled,
      showLabels: overlays.points,
    },
    theme,
  )

  drawOutline(
    ctx,
    viewport,
    {
      contours: scene.contours,
      filled: overlays.filled,
      showPoints: overlays.points,
      selected: options.selected,
    },
    theme,
  )

  if (overlays.anchors)
    drawAnchors(
      ctx,
      viewport,
      { anchors: scene.anchors, showLabels: true },
      theme,
    )

  if (overlays.rulers)
    drawRulers(ctx, viewport, { pointer: options.pointer }, theme)

  drawEditing(ctx, viewport, options.editing ?? NO_OVERLAY, theme)
  drawToolOverlay(ctx, viewport, options.tool ?? NO_TOOL_OVERLAY, theme)
  drawHud(
    ctx,
    viewport,
    options.pointerScreen ?? null,
    options.hud ?? null,
    theme,
  )
}
