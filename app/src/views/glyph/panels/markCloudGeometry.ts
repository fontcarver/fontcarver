import type { Anchor, GlyphCell, GlyphDetail } from '../../../api'

type Bounds = [number, number, number, number]

/** A mark outline and the font-space translation that attaches it to a base. */
export interface MarkAttachment {
  name: string
  path: string
  bbox: Bounds | null
  anchor: Anchor
  dx: number
  dy: number
}

function validBounds(bounds: Bounds | null): bounds is Bounds {
  return (
    bounds !== null &&
    bounds.every(Number.isFinite) &&
    bounds[0] <= bounds[2] &&
    bounds[1] <= bounds[3]
  )
}

/**
 * Match a base anchor to its underscore mark anchor in the exact master layer.
 * The caller must request the cell's path and bounds for the same master.
 */
export function matchMarkAttachment(
  baseAnchor: Anchor,
  detail: GlyphDetail,
  cell: GlyphCell,
  master: string,
): MarkAttachment | null {
  if (
    !baseAnchor.name.trim() ||
    baseAnchor.name.startsWith('_') ||
    !Number.isFinite(baseAnchor.x) ||
    !Number.isFinite(baseAnchor.y)
  ) {
    return null
  }

  const layer = detail.glyph.layers.find(
    ({ kind }) => 'Master' in kind && kind.Master === master,
  )
  const anchor = layer?.anchors.find(
    ({ name }) => name === `_${baseAnchor.name}`,
  )
  if (
    !layer ||
    !anchor ||
    !Number.isFinite(anchor.x) ||
    !Number.isFinite(anchor.y)
  ) {
    return null
  }

  const path =
    detail.paths.find((entry) => entry.layer === layer.id)?.flattened ??
    cell.path
  const dx = baseAnchor.x - anchor.x
  const dy = baseAnchor.y - anchor.y
  if (!path?.trim() || !Number.isFinite(dx) || !Number.isFinite(dy)) {
    return null
  }

  return {
    name: detail.glyph.name,
    path,
    bbox: validBounds(cell.bbox) ? cell.bbox : null,
    anchor,
    dx,
    dy,
  }
}

/**
 * Frame base and translated mark ink with six percent UPEM padding. The returned
 * SVG viewBox expects paths in a child group with `transform="scale(1,-1)"`.
 * Missing ink bounds use the base metrics and space around the attachment point.
 */
export function markCloudViewBox(
  baseBox: Bounds | null,
  baseWidth: number,
  upem: number,
  attachment: MarkAttachment,
): string {
  const em = Number.isFinite(upem) && upem > 0 ? upem : 1000
  const width =
    Number.isFinite(baseWidth) && baseWidth >= 0 ? baseWidth : em * 0.6
  const base: Bounds = validBounds(baseBox)
    ? baseBox
    : [0, -em * 0.2, width, em * 0.8]
  const { anchor, dx, dy } = attachment
  const mark: Bounds = validBounds(attachment.bbox)
    ? attachment.bbox
    : [
        anchor.x - em * 0.25,
        anchor.y - em * 0.25,
        anchor.x + em * 0.25,
        anchor.y + em * 0.25,
      ]
  const translated: Bounds = [
    mark[0] + dx,
    mark[1] + dy,
    mark[2] + dx,
    mark[3] + dy,
  ]
  const ink: Bounds = validBounds(translated)
    ? [
        Math.min(base[0], translated[0]),
        Math.min(base[1], translated[1]),
        Math.max(base[2], translated[2]),
        Math.max(base[3], translated[3]),
      ]
    : base
  const padding = Math.max(1, em * 0.06)
  const viewBox = [
    ink[0] - padding,
    -ink[3] - padding,
    ink[2] - ink[0] + padding * 2,
    ink[3] - ink[1] + padding * 2,
  ]
  // Extreme but finite font values can overflow while forming the viewport.
  return viewBox.every(Number.isFinite)
    ? viewBox.join(' ')
    : '-60 -860 720 1120'
}
