import { useEffect, useState } from 'react'

import { call, type DocId, type SpacingMatrix } from '../../api'

/** One answered `metrics.spacing` request, tagged with the query it answers. */
interface Answer {
  identity: string
  data: SpacingMatrix | null
  failed: boolean
}

/**
 * Bounded, cancellable spacing rows for one run, master and beam height.
 *
 * Every argument is part of the identity, so a superseded reply is discarded
 * instead of showing numbers for the previous text or master, and the panel
 * shows "loading" rather than stale values while a new answer is in flight.
 * `enabled` keeps the request from being made at all while the panel is closed.
 */
export function useSpacingMatrix(
  doc: DocId,
  master: string | null,
  generation: number,
  glyphs: readonly string[],
  beam: number | null,
  enabled: boolean,
  revision: number,
): Answer | null {
  const identity = JSON.stringify([
    doc,
    master,
    generation,
    glyphs,
    beam,
    enabled,
    revision,
  ])
  const [answer, setAnswer] = useState<Answer | null>(null)
  useEffect(() => {
    if (!enabled || glyphs.length === 0) return
    let current = true
    void call('metrics.spacing', {
      doc,
      master,
      glyphs: [...glyphs],
      beam,
    }).then(
      (data) => {
        if (current) setAnswer({ identity, data, failed: false })
      },
      () => {
        if (current) setAnswer({ identity, data: null, failed: true })
      },
    )
    return () => {
      current = false
    }
    // `identity` is every request argument plus the refresh counter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [identity])
  return answer?.identity === identity ? answer : null
}
