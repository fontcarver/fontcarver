/**
 * Composes one frame of the glyph editor from the individual drawing layers.
 *
 * The order is deliberate: metrics and guides sit behind the glyph, components
 * behind the layer's own contours, and anchors and rulers in front of
 * everything. Keeping the composition in one plain function means the React
 * component only has to decide *when* to draw, never *what*.
 */

import type { Viewport } from '../../canvas/Viewport'
import { drawSourceImage, type SourceImageDraw } from '../../canvas/sourceImage'

import { drawAnchors } from './drawAnchors'
import { drawAnnotations } from './drawAnnotations'
import { editorMetadata } from '../../editor/editorMetadata'
import { IDENTITY } from '../../editor/affine'
import { drawComponents, type ComponentDraw } from './drawComponents'
import { drawEditing, NO_OVERLAY, type EditingOverlay } from './drawEditing'
import { drawGuides } from './drawGuides'
import { drawHud } from './drawHud'
import { drawMetrics, type MetricLabels } from './drawMetrics'
import { drawOutline } from './drawOutline'
import {
  drawVariationPreview,
  type VariationCanvasPath,
} from './drawVariationPreview'
import { drawGrid, drawToolOverlay } from './drawTools'
import { drawRulers } from './drawRulers'
import { drawUnderlays, NO_UNDERLAY, type UnderlayData } from './drawUnderlays'
import { drawTextRun } from '../text/drawTextRun'
import type { SurroundingGlyph } from '../text/textCanvas'
import type { GlyphScene } from './glyphScene'
import type { GlyphTheme } from './glyphTheme'
import type { GuideOwner } from './guideIdentity'
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
  focusedContours?: ReadonlySet<number> | null
  glyphLocked?: boolean
  theme: GlyphTheme
  labels: MetricLabels
  overlays: GlyphOverlays
  /** Component-flattened outline of the shown layer, in font units. */
  flattened: Path2D | null
  /** The other masters and the background layer, drawn behind everything. */
  underlay?: UnderlayData
  /** Resolved PNG source, placed in font coordinates behind the outline. */
  sourceImage?: SourceImageDraw | null
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
  /** Index of the component the pointer is over, for the locked hatch. */
  hoveredComponent?: number | null
  /** Owner-scoped identities of guides shown as measurement guides. */
  measurementGuides?: ReadonlySet<string>
  /** Document/layer/master identity of the guide lists in this scene. */
  guideOwner?: GuideOwner
  /** Cursor-HUD text, or `null` when no gesture is running. */
  hud?: string | null
  /** Pointer position in canvas pixels, where the HUD is anchored. */
  pointerScreen?: { x: number; y: number } | null
  /** Transient geometry the active drawing tool is showing. */
  tool?: ToolOverlay
  /** Grid step in font units; zero draws no grid. */
  grid?: number
  /** Editable on-curve marker diameter in CSS pixels. */
  nodeSize?: number
  /** Editable off-curve marker diameter in CSS pixels. */
  handleSize?: number
  /** Passive outlines in a shaped Text run, relative to the active glyph. */
  surrounding?: readonly SurroundingGlyph[]
  /** Text mode has no editable foreground glyph. */
  hideActive?: boolean
  /** Live interpolation at the inspector's selected axis location. */
  variationPreview?: VariationCanvasPath | null
}

/** The vertical metrics a node can sit on (design spec §4.1 item 9). */
function verticalMetricsOf(scene: GlyphScene): number[] {
  const { metrics } = scene
  const values = [
    0,
    metrics.xHeight,
    metrics.capHeight,
    metrics.ascender,
    metrics.descender,
  ]
  if (metrics.verticalOrigin !== null) values.push(metrics.verticalOrigin)
  return [...new Set(values.filter((value) => Number.isFinite(value)))]
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

  if (!options.hideActive && overlays.background && options.sourceImage)
    drawSourceImage(ctx, viewport, options.sourceImage)

  if (!options.hideActive && overlays.metrics)
    drawMetrics(
      ctx,
      viewport,
      { metrics: scene.metrics, labels: options.labels, showLabels: true },
      theme,
    )

  if (!options.hideActive && overlays.guides)
    drawGuides(
      ctx,
      viewport,
      {
        guides: scene.guides,
        showLabels: true,
        selected: options.selected,
        measurement: options.measurementGuides,
        owner: options.guideOwner,
        contours: scene.contours,
      },
      theme,
    )

  const underlay = options.underlay ?? NO_UNDERLAY
  if (!options.hideActive)
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

  drawTextRun(ctx, viewport, options.surrounding ?? [], theme.outline)

  if (!options.hideActive) {
    const effects =
      (scene.layer?.effects?.some((effect) => effect.enabled) ?? false) ||
      (scene.layer?.contours.some((c) =>
        c.points.some((p) => (p.corner_radius ?? 0) !== 0),
      ) ??
        false)
    if (effects && options.flattened) {
      ctx.save()
      ctx.translate(viewport.offset.x, viewport.offset.y)
      ctx.scale(viewport.scale, -viewport.scale)
      if (overlays.filled) {
        ctx.fillStyle = theme.fill
        ctx.fill(options.flattened, 'nonzero')
      } else {
        ctx.strokeStyle = theme.outline
        ctx.lineWidth = 1 / viewport.scale
        ctx.stroke(options.flattened)
      }
      ctx.restore()
    }
    drawComponents(
      ctx,
      viewport,
      {
        path: options.flattened,
        hidePaths: effects,
        components:
          options.components ??
          scene.components.map((component) => ({
            component,
            path: null,
            transform: IDENTITY,
          })),
        bounds: scene.componentBounds,
        hovered: options.hoveredComponent ?? null,
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
        focusedContours: options.focusedContours,
        lockedPaths: editorMetadata(scene.layer?.user_data).lockedPaths,
        hidePath: effects,
        filled: overlays.filled,
        showPoints: overlays.points && !options.glyphLocked,
        selected: options.selected,
        verticalMetrics: verticalMetricsOf(scene),
        nodeSize: options.nodeSize,
        handleSize: options.handleSize,
      },
      theme,
    )
  }

  if (!options.hideActive && options.variationPreview)
    drawVariationPreview(ctx, viewport, options.variationPreview, theme)

  if (!options.hideActive && overlays.anchors)
    drawAnchors(
      ctx,
      viewport,
      { anchors: scene.anchors, showLabels: true },
      theme,
    )

  if (overlays.rulers)
    drawRulers(ctx, viewport, { pointer: options.pointer }, theme)

  if (!options.hideActive)
    drawAnnotations(
      ctx,
      viewport,
      editorMetadata(scene.layer?.user_data).annotations,
      theme,
    )

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
