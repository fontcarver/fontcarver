/**
 * Live resolution of one glyph's metrics keys.
 *
 * `glyphs.outline` already carries `resolved_metrics`, but only as of the last
 * time *this* glyph was fetched: a key such as `=H` follows another glyph, and
 * editing `H` does not refetch the layer of the glyph that references it. The
 * panel therefore asks `doc.metrics_resolve` for the live answer whenever the
 * formulas, the glyph or the master change, and falls back to the outline's
 * own copy until that answers.
 */

import { useEffect, useState } from 'react'

import {
  call,
  onDocumentEvent,
  type DocId,
  type GlyphId,
  type MasterId,
  type MetricsKeys,
  type ResolvedMetrics,
} from '../../../api'

/** The resolved metrics of one glyph, refreshed on every document change. */
export function useResolvedMetrics(
  doc: DocId,
  gid: GlyphId,
  master: MasterId | null,
  keys: MetricsKeys,
  fallback: ResolvedMetrics,
): ResolvedMetrics {
  const [resolved, setResolved] = useState<ResolvedMetrics | null>(null)
  const [generation, setGeneration] = useState(0)
  // A formula names other glyphs, so the request has to be reissued whenever
  // any of them could have moved; the signature keeps it to real changes.
  const signature = `${doc}:${gid}:${master ?? ''}:${keys.left ?? ''}:${
    keys.right ?? ''
  }:${keys.width ?? ''}`

  useEffect(() => {
    let cancelled = false
    void call('doc.metrics_resolve', { doc, gid, master })
      .then((answer) => {
        if (!cancelled) setResolved(answer)
      })
      .catch(() => {
        // A cyclic or malformed key makes the kernel refuse; the stored
        // sidebearings stay on screen rather than the panel going blank.
        if (!cancelled) setResolved(null)
      })
    return () => {
      cancelled = true
    }
    // `signature` stands in for doc, gid, master and the three formulas.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature, generation])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc) return
        if (!event.change.metrics && !event.change.glyphs.includes(gid)) return
        setGeneration((value) => value + 1)
      }),
    [doc, gid],
  )

  return resolved ?? fallback
}

/** Whether a resolved value differs from what the layer actually stores. */
export function outOfSync(
  resolved: ResolvedMetrics,
  stored: { lsb: number; rsb: number; width: number },
  tolerance = 0.5,
): boolean {
  return (
    Math.abs(resolved.lsb - stored.lsb) > tolerance ||
    Math.abs(resolved.rsb - stored.rsb) > tolerance ||
    Math.abs(resolved.width - stored.width) > tolerance
  )
}
