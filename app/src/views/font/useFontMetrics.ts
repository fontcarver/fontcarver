/**
 * Vertical metrics of the selected master.
 *
 * The grid fits every glyph into the same em box, so it needs the master's
 * ascender and descender. `doc.summary` carries only the unit size, so the
 * metrics come from the shared `doc.font_info` cache, which the glyph editor
 * and the Font Info tab read as well.
 */

import { useEffect, useState } from 'react'

import type { DocId, MasterId } from '../../api'
import { emBoxFor, type EmBox } from '../../canvas/glyphPaint'
import {
  cachedFontInfo,
  forgetFontInfo,
  loadFontInfo,
} from '../../stores/fontInfoCache'

/** Ascender and descender per master of one document. */
type MasterMetrics = Map<string, { ascender: number; descender: number }>

function metricsOf(doc: DocId): MasterMetrics | null {
  const info = cachedFontInfo(doc)
  if (!info) return null
  return new Map(
    info.masters.map((master) => [
      master.id,
      {
        ascender: master.metrics.ascender,
        descender: master.metrics.descender,
      },
    ]),
  )
}

/** Forgets one document's cached font info; called when the document closes. */
export function forgetFontMetrics(doc: DocId): void {
  forgetFontInfo(doc)
}

/**
 * The em box for one master. Until the metrics arrive — and for fonts that
 * report none — the conventional 80 % / 20 % split of the em is used.
 */
export function useFontMetrics(
  doc: DocId | null,
  master: MasterId | null,
  unitsPerEm: number,
): EmBox {
  const masterKey = master ?? ''
  const [metrics, setMetrics] = useState<MasterMetrics | null>(() =>
    doc === null ? null : metricsOf(doc),
  )

  useEffect(() => {
    if (doc === null) return
    let cancelled = false
    void loadFontInfo(doc)
      .then(() => {
        if (!cancelled) setMetrics(metricsOf(doc))
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
