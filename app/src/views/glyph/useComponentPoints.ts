/**
 * The ghost nodes of a layer's components (design spec §4.1 item 14, §4.2).
 *
 * `glyphs.flattened_components` returns every point of the resolved component
 * outlines, which is what the pointer snaps to and what the canvas draws as
 * faint ghosts when it comes near one. A layer with no components asks for
 * nothing, and a failure is silent: ghosts are a convenience, not a feature the
 * editor depends on.
 */

import { useEffect, useState } from 'react'

import { call, type DocId, type GlyphId, type LayerId } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

const NONE: readonly Point2[] = []

/** Fetches the component ghost nodes of one layer. */
export function useComponentPoints(
  doc: DocId,
  gid: GlyphId | undefined,
  layer: LayerId | null,
  componentCount: number,
): readonly Point2[] {
  // The points are stored under the key they were fetched for, so a layer
  // change is answered with `NONE` on the very same render rather than by a
  // second pass through the effect.
  const [loaded, setLoaded] = useState<{
    key: string
    points: readonly Point2[]
  }>({ key: '', points: NONE })
  const wanted =
    gid === undefined || layer === null || componentCount === 0
      ? ''
      : `${doc}:${gid}:${layer}:${componentCount}`

  useEffect(() => {
    if (wanted === '') return
    let cancelled = false
    void call('glyphs.flattened_components', {
      doc,
      gid: gid as GlyphId,
      layer,
    })
      .then((result) => {
        if (cancelled) return
        setLoaded({
          key: wanted,
          points: result.points.map(([x, y]) => ({ x, y })),
        })
      })
      .catch(() => {
        // Ghost nodes are a convenience; a failure just leaves them out.
        if (!cancelled) setLoaded({ key: wanted, points: NONE })
      })
    return () => {
      cancelled = true
    }
    // `wanted` names every input the request depends on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wanted])

  return loaded.key === wanted ? loaded.points : NONE
}
