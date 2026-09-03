/**
 * Vertical metrics of the selected master.
 *
 * The grid fits every glyph into the same em box, so it needs the master's
 * ascender and descender. `doc.summary` carries only the unit size, so the
 * metrics come from `doc.font_info`, requested once per document and cached
 * for the lifetime of the session.
 */

import { useEffect, useState } from 'react'

import { call, type DocId, type MasterId } from '../../api'
import { emBoxFor, type EmBox } from '../../canvas/glyphPaint'
import { onDocumentForgotten } from '../../stores/documentStore'

/** Ascender and descender per master of one document. */
type MasterMetrics = Map<string, { ascender: number; descender: number }>

const cache = new Map<DocId, MasterMetrics>()
const inFlight = new Map<DocId, Promise<MasterMetrics>>()

async function loadMetrics(doc: DocId): Promise<MasterMetrics> {
  const cached = cache.get(doc)
  if (cached) return cached
  const pending = inFlight.get(doc)
  if (pending) return pending

  const request = call('doc.font_info', { doc })
    .then((info) => {
      const metrics: MasterMetrics = new Map(
        info.masters.map((master) => [
          master.id.toString(),
          {
            ascender: master.metrics.ascender,
            descender: master.metrics.descender,
          },
        ]),
      )
      cache.set(doc, metrics)
      return metrics
    })
    .finally(() => inFlight.delete(doc))

  inFlight.set(doc, request)
  return request
}

/** Forgets one document's metrics; called when the document closes. */
export function forgetFontMetrics(doc: DocId): void {
  cache.delete(doc)
}

onDocumentForgotten(forgetFontMetrics)

/**
 * The em box for one master. Until the metrics arrive — and for fonts that
 * report none — the conventional 80 % / 20 % split of the em is used.
 */
export function useFontMetrics(
  doc: DocId | null,
  master: MasterId | null,
  unitsPerEm: number,
): EmBox {
  const masterKey = master?.toString() ?? ''
  const [metrics, setMetrics] = useState<MasterMetrics | null>(() =>
    doc === null ? null : (cache.get(doc) ?? null),
  )

  useEffect(() => {
    if (doc === null) return
    let cancelled = false
    void loadMetrics(doc)
      .then((loaded) => {
        if (!cancelled) setMetrics(loaded)
      })
      .catch(() => {
        // Metrics are an optimisation; the fallback em box stays in use.
      })
    return () => {
      cancelled = true
    }
  }, [doc])

  const entry =
    metrics === null
      ? undefined
      : (metrics.get(masterKey) ?? [...metrics.values()][0])
  return emBoxFor(unitsPerEm, entry?.ascender, entry?.descender)
}
