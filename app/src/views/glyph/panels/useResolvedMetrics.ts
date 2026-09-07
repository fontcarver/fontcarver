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

import { useEffect, useRef, useState } from 'react'

import {
  call,
  onDocumentEvent,
  type DocId,
  type GlyphId,
  type MasterId,
  type MetricsKeys,
  type ResolvedMetrics,
} from '../../../api'
import { useEditorStore } from '../../../stores/editorStore'

export interface ResolvedMetricsState {
  value: ResolvedMetrics
  status: 'pending' | 'ready' | 'error'
}

interface Resolution {
  identity: string
  value: ResolvedMetrics | null
  status: 'ready' | 'error'
}

/** The resolved metrics of one glyph, refreshed on every document change. */
export function useResolvedMetrics(
  doc: DocId,
  gid: GlyphId,
  master: MasterId | null,
  keys: MetricsKeys,
  fallback: ResolvedMetrics,
): ResolvedMetricsState {
  const [resolved, setResolved] = useState<Resolution | null>(null)
  const [generation, setGeneration] = useState(0)
  const editing = useEditorStore((state) => state.editing)
  const deferred = useRef(false)
  // A formula names other glyphs, so the request has to be reissued whenever
  // any of them could have moved; the signature keeps it to real changes.
  const signature = `${doc}:${gid}:${master ?? ''}:${keys.left ?? ''}:${
    keys.right ?? ''
  }:${keys.width ?? ''}`
  const identity = `${signature}:${generation}`

  useEffect(() => {
    let cancelled = false
    void call('doc.metrics_resolve', { doc, gid, master })
      .then((answer) => {
        if (!cancelled)
          setResolved({ identity, value: answer, status: 'ready' })
      })
      .catch(() => {
        // A cyclic or malformed key makes the kernel refuse; the stored
        // sidebearings stay on screen rather than the panel going blank.
        if (!cancelled) setResolved({ identity, value: null, status: 'error' })
      })
    return () => {
      cancelled = true
    }
    // `identity` stands in for the request arguments and document generation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [identity])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc) return
        if (!event.change.metrics && !event.change.glyphs.includes(gid)) return
        // A sidebearing drag applies once per animation frame, so the kernel
        // already holds geometry the fetched layer deliberately does not.
        // Resolving against that would flash "out of sync" for the length of
        // the drag; the answer that matters is the one after it ends.
        if (useEditorStore.getState().editing) {
          deferred.current = true
          return
        }
        setGeneration((value) => value + 1)
      }),
    [doc, gid],
  )

  useEffect(() => {
    if (editing || !deferred.current) return
    deferred.current = false
    setGeneration((value) => value + 1)
  }, [editing])

  if (resolved?.identity !== identity)
    return { value: fallback, status: 'pending' }
  return {
    value: resolved.value ?? fallback,
    status: resolved.status,
  }
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
