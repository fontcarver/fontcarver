/**
 * Source clusters of one shaped run, for the cluster inspector.
 *
 * The shaper reports a UTF-8 source offset per glyph; a cluster is the set of
 * glyphs that share one offset, which is what "which glyphs came from which
 * characters" means. Visual order is preserved, so a right-to-left run reads
 * the way it is drawn while its source offsets still name the original text.
 */

import type { PositionedGlyph } from './textLayout'

/** One glyph of a cluster, with the geometry the inspector shows. */
export interface ClusterGlyph {
  name: string | null
  x: number
  y: number
  xAdvance: number
  yAdvance: number
}

/** One source cluster in visual order. */
export interface ShapedCluster {
  /** Ordinal in visual order, which is what the inspector numbers. */
  index: number
  /** UTF-8 byte offset the shaper reported for every glyph in the cluster. */
  sourceByte: number
  /** The source characters of the cluster, or an empty string if unmappable. */
  source: string
  glyphs: ClusterGlyph[]
}

const encoder = new TextEncoder()
const decoder = new TextDecoder()

/**
 * Groups a run into clusters and maps each back to its source characters.
 *
 * A cluster ends at the next larger source offset in the *text*, not the next
 * glyph, so the mapping is the same for a left-to-right and a right-to-left run.
 */
export function textClusters(
  text: string,
  glyphs: readonly PositionedGlyph[],
): ShapedCluster[] {
  const bytes = encoder.encode(text)
  const boundaries = [...new Set(glyphs.map((glyph) => glyph.sourceByte))].sort(
    (left, right) => left - right,
  )
  const endOf = new Map<number, number>()
  boundaries.forEach((start, position) =>
    endOf.set(start, boundaries[position + 1] ?? bytes.length),
  )
  const clusters: ShapedCluster[] = []
  for (const glyph of glyphs) {
    const last = clusters.at(-1)
    const entry: ClusterGlyph = {
      name: glyph.name,
      x: glyph.x,
      y: glyph.y,
      xAdvance: glyph.xAdvance,
      yAdvance: glyph.yAdvance,
    }
    if (last && last.sourceByte === glyph.sourceByte) {
      last.glyphs.push(entry)
      continue
    }
    const start = glyph.sourceByte
    const end = endOf.get(start) ?? bytes.length
    clusters.push({
      index: clusters.length,
      sourceByte: start,
      source:
        start >= 0 && start <= bytes.length && end >= start
          ? decoder.decode(bytes.subarray(start, Math.min(end, bytes.length)))
          : '',
      glyphs: [entry],
    })
  }
  return clusters
}
