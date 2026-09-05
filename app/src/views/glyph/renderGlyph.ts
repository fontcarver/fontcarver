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
import { drawComponents } from './drawComponents'
import { drawEditing, NO_OVERLAY, type EditingOverlay } from './drawEditing'
import { drawGuides } from './drawGuides'
import { drawHud } from './drawHud'
import { drawMetrics, type MetricLabels } from './drawMetrics'
import { drawOutline } from './drawOutline'
import { drawRulers } from './drawRulers'
import { drawUnderlays, NO_UNDERLAY, type UnderlayData } from './drawUnderlays'
import type { GlyphScene } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'

/** Which overlays the editor is currently showing. */
export interface GlyphOverlays {
  /** Fill the outline instead of stroking it. */
  filled: boolean
  points: boolean
  metrics: boolean
  anchors: boolean
  guides: boolean
  rulers: boolean
  /** The glyph's background layer for this master (design spec §4.1 item 5). */
  background: boolean
  /** The same glyph in every other master (design spec §4.1 item 4). */
  otherMasters: boolean
}

/** Every overlay on, which is the default for a read-only glyph tab. */
export const DEFAULT_OVERLAYS: GlyphOverlays = {
  filled: false,
  points: true,
  metrics: true,
  anchors: true,
  guides: true,
  rulers: true,
  background: true,
  otherMasters: false,
}

/** Everything one frame needs. */
export interface RenderGlyphOptions {
  scene: GlyphScene
  theme: GlyphTheme
  labels: MetricLabels
  overlays: GlyphOverlays
  /** Component-flattened outline of the shown layer, in font units. */
  flattened: Path2D | null
  /** The other masters and the background layer, drawn behind everything. */
  underlay?: UnderlayData
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

  if (overlays.metrics)
    drawMetrics(
      ctx,
      viewport,
      { metrics: scene.metrics, labels: options.labels, showLabels: true },
      theme,
    )

  if (overlays.guides)
    drawGuides(ctx, viewport, { guides: scene.guides, showLabels: true }, theme)

  const underlay = options.underlay ?? NO_UNDERLAY
  drawUnderlays(
    ctx,
    viewport,
    {
      background: overlays.background ? underlay.background : null,
      backgroundPoints: overlays.background ? underlay.backgroundPoints : [],
      otherMasters: overlays.otherMasters ? underlay.otherMasters : [],
    },
    theme,
  )

  drawComponents(
    ctx,
    viewport,
    {
      path: options.flattened,
      components: scene.components,
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
  drawHud(
    ctx,
    viewport,
    options.pointerScreen ?? null,
    options.hud ?? null,
    theme,
  )
}
