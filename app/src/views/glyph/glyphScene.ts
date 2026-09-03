/**
 * Turns a `glyphs.detail` response into the flat, drawable description the
 * canvas layers consume.
 *
 * Everything here is pure so the geometry decisions — which layer is shown,
 * where the sidebearings are, what the initial view frames — are unit testable
 * without a canvas. Font units and y-up are preserved throughout; the viewport
 * is the only place that knows about pixels.
 */

import type {
  Anchor,
  Component,
  Contour,
  Glyph,
  GlyphCell,
  GlyphDetail,
  Guide,
  Layer,
  LayerKind,
  Master,
  MasterId,
} from '../../api'
import type { FontBox } from '../../canvas/Viewport'

/** Vertical and horizontal metrics the editor draws for one layer. */
export interface SceneMetrics {
  upem: number
  ascender: number
  descender: number
  xHeight: number
  capHeight: number
  italicAngle: number
  /** Horizontal advance of the shown layer. */
  width: number
  /** Left sidebearing, or `null` for a glyph with no outline. */
  lsb: number | null
  /** Right sidebearing, or `null` for a glyph with no outline. */
  rsb: number | null
  verticalOrigin: number | null
  verticalWidth: number | null
}

/** A guide plus the scope it came from. */
export interface SceneGuide {
  guide: Guide
  /** Master guides are font-wide; layer guides belong to this glyph alone. */
  scope: 'master' | 'layer'
}

/** Everything one glyph tab draws. */
export interface GlyphScene {
  name: string
  unicodes: number[]
  /** The layer being shown, or `null` for a glyph with no layers at all. */
  layer: Layer | null
  layerName: string
  contours: Contour[]
  components: Component[]
  anchors: Anchor[]
  guides: SceneGuide[]
  metrics: SceneMetrics
  /** Exact outline bounds from the kernel, or `null` when the glyph is blank. */
  bbox: FontBox | null
  /** Component-flattened SVG path data of the shown layer. */
  flattened: string | null
}

/** The master a layer belongs to, or `null` for a free-standing custom layer. */
export function layerMasterId(kind: LayerKind): string | null {
  if ('Master' in kind) return kind.Master
  if ('Brace' in kind) return kind.Brace.master
  if ('Bracket' in kind) return kind.Bracket.master
  if ('Background' in kind) return kind.Background
  if ('Backup' in kind) return kind.Backup.of
  return null
}

/** Whether a layer is the plain master layer of `master`. */
function isMasterLayer(layer: Layer, master: string | null): boolean {
  if (!('Master' in layer.kind)) return false
  return master === null || layer.kind.Master === master
}

/**
 * The layer to show for a master: its own master layer when there is one, the
 * first master layer otherwise, and finally whatever layer exists at all.
 */
export function activeLayer(
  glyph: Glyph,
  master: MasterId | null,
): Layer | null {
  const wanted = master
  return (
    glyph.layers.find((layer) => isMasterLayer(layer, wanted)) ??
    glyph.layers.find((layer) => isMasterLayer(layer, null)) ??
    glyph.layers[0] ??
    null
  )
}

/** Conventional metrics for a font that reports none. */
function fallbackMetrics(upem: number): {
  ascender: number
  descender: number
  xHeight: number
  capHeight: number
} {
  return {
    ascender: upem * 0.8,
    descender: -upem * 0.2,
    xHeight: upem * 0.5,
    capHeight: upem * 0.7,
  }
}

export interface BuildSceneInput {
  detail: GlyphDetail
  /** The grid cell of the same glyph, which carries the exact outline bounds. */
  cell: GlyphCell | null
  master: Master | null
  masterId: MasterId | null
  unitsPerEm: number
}

/** Builds the drawable scene for one glyph and master. */
export function buildScene({
  detail,
  cell,
  master,
  masterId,
  unitsPerEm,
}: BuildSceneInput): GlyphScene {
  const glyph = detail.glyph
  const layer = activeLayer(glyph, masterId)
  const upem = unitsPerEm > 0 ? unitsPerEm : 1000
  const fallback = fallbackMetrics(upem)
  const metrics = master?.metrics

  const width = layer?.width ?? 0
  const bounds = cell?.bbox ?? null
  const bbox: FontBox | null = bounds
    ? { x0: bounds[0], y0: bounds[1], x1: bounds[2], y1: bounds[3] }
    : null

  const flattened =
    layer === null
      ? null
      : (detail.paths.find((path) => path.layer === layer.id)?.flattened ??
        null)

  const guides: SceneGuide[] = [
    ...(master?.guides ?? []).map((guide): SceneGuide => ({
      guide,
      scope: 'master',
    })),
    ...(layer?.guides ?? []).map((guide): SceneGuide => ({
      guide,
      scope: 'layer',
    })),
  ]

  return {
    name: glyph.name,
    unicodes: glyph.unicodes,
    layer,
    // Imported fonts leave the layer unnamed; the master names it instead.
    layerName: layer?.name || master?.name || '',
    contours: layer?.contours ?? [],
    components: layer?.components ?? [],
    anchors: layer?.anchors ?? [],
    guides,
    metrics: {
      upem,
      ascender: metrics?.ascender ?? fallback.ascender,
      descender: metrics?.descender ?? fallback.descender,
      xHeight: metrics?.x_height ?? fallback.xHeight,
      capHeight: metrics?.cap_height ?? fallback.capHeight,
      italicAngle: metrics?.italic_angle ?? 0,
      width,
      lsb: bbox ? bbox.x0 : null,
      rsb: bbox ? width - bbox.x1 : null,
      verticalOrigin:
        layer?.vertical_origin ?? metrics?.default_vertical_origin ?? null,
      verticalWidth:
        layer?.vertical_width ?? metrics?.default_vertical_advance ?? null,
    },
    bbox,
    flattened,
  }
}

/**
 * The box the initial view frames: the advance box of the layer together with
 * whatever the outline actually covers, so overshooting or negatively bearing
 * glyphs are still fully visible.
 */
export function sceneBox(scene: GlyphScene): FontBox {
  const { metrics, bbox } = scene
  const box: FontBox = {
    x0: 0,
    y0: Math.min(metrics.descender, 0),
    x1: metrics.width > 0 ? metrics.width : metrics.upem,
    y1: Math.max(metrics.ascender, metrics.upem * 0.5),
  }
  if (!bbox) return box
  return {
    x0: Math.min(box.x0, bbox.x0),
    y0: Math.min(box.y0, bbox.y0),
    x1: Math.max(box.x1, bbox.x1),
    y1: Math.max(box.y1, bbox.y1),
  }
}

/** `U+0041`-style rendering of the glyph's first Unicode scalar values. */
export function formatUnicodes(unicodes: readonly number[]): string {
  if (unicodes.length === 0) return '—'
  return unicodes
    .map((value) => `U+${value.toString(16).toUpperCase().padStart(4, '0')}`)
    .join(' ')
}

/** Total number of on-curve and off-curve points in a scene. */
export function pointCount(scene: GlyphScene): number {
  return scene.contours.reduce(
    (total, contour) => total + contour.points.length,
    0,
  )
}
