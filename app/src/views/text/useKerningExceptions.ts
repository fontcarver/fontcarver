import { useEffect, useMemo, useState } from 'react'

import { call, type DocId, type KerningBoundaries } from '../../api'
import type { TextGap } from './textGaps'

/** Largest number of boundaries one request asks about, mirroring the kernel. */
export const KERNING_BOUNDARY_LIMIT = 256

/** The editable boundaries of a run, as the pairs the kernel is asked about. */
export function boundaryPairs(
  gaps: readonly TextGap[],
  limit = KERNING_BOUNDARY_LIMIT,
): { index: number; pair: [string, string] }[] {
  const pairs: { index: number; pair: [string, string] }[] = []
  for (const gap of gaps) {
    if (gap.reason !== null || gap.left === null || gap.right === null) continue
    pairs.push({ index: gap.index, pair: [gap.left, gap.right] })
    if (pairs.length >= limit) break
  }
  return pairs
}

/**
 * Which boundaries of the run are kerning exceptions.
 *
 * Row 42 asks for exceptions to be visible in the canvas, and an exception is a
 * property of the *winning rule*, which only the kernel knows. Resolving one
 * boundary at a time would be a request per letter on every keystroke, so the
 * whole run is asked once, at one generation, and a superseded answer is
 * discarded rather than ringing the wrong pairs. An unreadable boundary is
 * simply not marked.
 */
export function useKerningExceptions({
  doc,
  master,
  generation,
  gaps,
  vertical,
  enabled,
}: {
  doc: DocId
  master: string | null
  generation: number
  gaps: readonly TextGap[]
  vertical: boolean
  enabled: boolean
}): ReadonlySet<number> {
  const pairs = useMemo(() => boundaryPairs(gaps), [gaps])
  const identity = JSON.stringify([doc, master, generation, vertical, pairs])
  const [answer, setAnswer] = useState<{
    identity: string
    indexes: ReadonlySet<number>
  } | null>(null)
  useEffect(() => {
    if (!enabled || !master || pairs.length === 0) return
    let current = true
    void call('kerning.boundaries', {
      doc,
      master,
      vertical,
      pairs: pairs.map((entry) => entry.pair),
    }).then(
      (result: KerningBoundaries) => {
        if (!current) return
        const indexes = new Set<number>()
        result.items.forEach((item, position) => {
          const entry = pairs[position]
          if (item.exception && entry) indexes.add(entry.index)
        })
        setAnswer({ identity, indexes })
      },
      () => {
        if (current) setAnswer({ identity, indexes: new Set() })
      },
    )
    return () => {
      current = false
    }
    // `identity` carries every request argument.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [identity, enabled])
  return answer?.identity === identity ? answer.indexes : EMPTY
}

const EMPTY: ReadonlySet<number> = new Set()
