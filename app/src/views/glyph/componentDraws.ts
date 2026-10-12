/**
 * What the canvas draws for each component of a layer.
 *
 * The kernel hands back one flattened path per component, already transformed
 * by the component's *stored* transform. A live gesture changes that transform
 * without changing the path, so what the canvas needs is the difference: the
 * affine that takes the path where it is to where the gesture is putting it.
 * For a move that is a translation; for a rotate or a scale it is the whole
 * transform, which is why this is an affine rather than a `dx`/`dy` pair.
 */

import type { ComponentView } from '../../api'
import type { Affine } from '../../editor/affine'
import {
  composeAffine,
  invertAffine,
  isInvertible,
  IDENTITY,
} from '../../editor/affine'

import type { ComponentDraw } from './drawComponents'

/** A parsed path per path string; malformed data is remembered as `null`. */
export type PathCache = Map<string, Path2D | null>

/** A parsed path for the given data, cached; malformed data yields `null`. */
export function parsePath(data: string, cache: PathCache): Path2D | null {
  const cached = cache.get(data)
  if (cached !== undefined) return cached
  let path: Path2D | null
  try {
    path = new Path2D(data)
  } catch {
    // Malformed path data must not take the whole frame down.
    path = null
  }
  cache.set(data, path)
  return path
}

/**
 * The transform that takes a path rendered under `stored` to `previewed`.
 *
 * A component whose stored transform collapses the base outline cannot be
 * undone, so there the delta falls back to the change in offset — the path is
 * degenerate either way, and a translation at least keeps its marker honest.
 */
function deltaOf(stored: Affine, previewed: Affine): Affine {
  if (stored === previewed) return IDENTITY
  if (!isInvertible(stored))
    return [1, 0, 0, 1, previewed[4] - stored[4], previewed[5] - stored[5]]
  return composeAffine(previewed, invertAffine(stored))
}

/**
 * The components as this frame draws them.
 *
 * `preview` is the geometry a gesture is showing, `source` what the kernel
 * last reported; they are the same list in the same order, so the delta for
 * component *i* is `preview[i].transform ∘ source[i].transform⁻¹`.
 */
export function componentDraws(
  source: readonly ComponentView[],
  preview: readonly ComponentView[] | null,
  cache: PathCache,
): readonly ComponentDraw[] {
  const shown = preview ?? source
  return shown.map((view, index) => {
    const base = source[index] ?? view
    return {
      component: view.component,
      path: parsePath(base.path, cache),
      transform: deltaOf(base.component.transform, view.component.transform),
    }
  })
}
