/**
 * Guides anchored to a sidebearing (audit row 30, design spec §6.1).
 *
 * A guide's stored `x` is absolute until it is anchored, and then it is a
 * distance from the left or the right edge of the layer's ink. The scene
 * resolves that into a position for drawing, hit testing, snapping and
 * measurement; this module is the other half — what the inspector needs to show
 * the stored offset, to move the anchor without moving the guide, and to name
 * the two edges.
 */

import type { Guide, GuideAnchor } from '../../api'

/** The three anchor choices, in the order the inspector lists them. */
export const GUIDE_ANCHORS: readonly GuideAnchor[] = ['Origin', 'Lsb', 'Rsb']

/** The ink edges an anchored guide measures from, in font units. */
export interface InkEdges {
  /** Left edge of the ink (the left sidebearing), or `null` when blank. */
  left: number | null
  /** Right edge of the ink, or `null` when the glyph is blank. */
  right: number | null
}

/** No ink, so every anchor behaves as an absolute position. */
export const NO_INK_EDGES: InkEdges = { left: null, right: null }

/** The font-space x an anchor measures from, or `null` when it cannot. */
export function anchorOrigin(
  anchor: GuideAnchor,
  edges: InkEdges,
): number | null {
  const edge =
    anchor === 'Lsb' ? edges.left : anchor === 'Rsb' ? edges.right : null
  return edge !== null && Number.isFinite(edge) ? edge : null
}

/** Where an anchored guide actually sits, in absolute font units. */
export function resolvedGuideX(guide: Guide, edges: InkEdges): number {
  const origin = anchorOrigin(guide.anchor, edges)
  return origin === null ? guide.x : origin + guide.x
}

/**
 * The same guide anchored differently, without moving it.
 *
 * Changing what a distance is measured from would otherwise teleport the guide,
 * which is never what the user meant: the offset is rebased so the line stays
 * exactly where it is and only its *relationship* to the glyph changes.
 */
export function reanchorGuide(
  guide: Guide,
  anchor: GuideAnchor,
  edges: InkEdges,
): Guide {
  if (anchor === guide.anchor) return guide
  const absolute = resolvedGuideX(guide, edges)
  const origin = anchorOrigin(anchor, edges)
  return { ...guide, anchor, x: origin === null ? absolute : absolute - origin }
}

/** The ink edges of one scene's metrics, as the inspector reads them. */
export function inkEdgesOf(metrics: {
  width: number
  lsb: number | null
  rsb: number | null
}): InkEdges {
  return {
    left: metrics.lsb,
    right: metrics.rsb === null ? null : metrics.width - metrics.rsb,
  }
}
