import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type MathConstruction,
  type MathPreview,
  type MathPreviewGlyph,
} from '../../api'
import { errorNotice } from '../../i18n/errors'
import styles from './math.module.css'

interface Placement {
  glyph: MathPreviewGlyph
  x: number
  y: number
}

/**
 * Parts are placed along the extension direction: horizontally from the
 * glyph origin, vertically from the ink bottom of each part.
 */
function placements(preview: MathPreview, vertical: boolean): Placement[] {
  const byName = new Map(preview.glyphs.map((g) => [g.name, g]))
  const single = preview.variant ?? preview.fallback
  if (single) {
    const glyph = byName.get(single)
    return glyph ? [{ glyph, x: 0, y: 0 }] : []
  }
  return (preview.assembly?.parts ?? []).flatMap((part) => {
    const glyph = byName.get(part.glyph)
    if (!glyph) return []
    return vertical
      ? [{ glyph, x: 0, y: part.offset - glyph.bounds[1] }]
      : [{ glyph, x: part.offset, y: 0 }]
  })
}

/** Size a construction the way a layout engine chooses variants or parts. */
export function MathAssemblyPreview({
  doc,
  glyph,
  vertical,
  construction,
  overlap,
  upem,
}: {
  doc: DocId
  glyph: string
  vertical: boolean
  construction: MathConstruction
  overlap: number
  upem: number
}) {
  const { t } = useTranslation()
  const [target, setTarget] = useState(upem * 2)
  const [preview, setPreview] = useState<MathPreview | null>(null)
  const [error, setError] = useState('')
  const request = JSON.stringify(construction)
  useEffect(() => {
    let stale = false
    const timer = window.setTimeout(() => {
      call('math.preview', {
        doc,
        glyph,
        vertical,
        target,
        construction: JSON.parse(request) as MathConstruction,
        min_connector_overlap: overlap,
      })
        .then((result) => {
          if (stale) return
          setPreview(result)
          setError('')
        })
        .catch((reason: unknown) => {
          if (stale) return
          const notice = errorNotice(reason, t)
          setPreview(null)
          setError(notice.details ?? notice.message)
        })
    }, 150)
    return () => {
      stale = true
      window.clearTimeout(timer)
    }
  }, [doc, glyph, vertical, target, request, overlap, t])
  const placed = preview ? placements(preview, vertical) : []
  const xs = placed.flatMap((p) => [
    p.x + p.glyph.bounds[0],
    p.x + p.glyph.bounds[2],
  ])
  const ys = placed.flatMap((p) => [
    p.y + p.glyph.bounds[1],
    p.y + p.glyph.bounds[3],
  ])
  const margin = upem * 0.1
  const x0 = Math.min(0, ...xs) - margin
  const x1 = Math.max(upem * 0.5, ...xs, vertical ? 0 : target) + margin
  const y0 = Math.min(0, ...ys) - margin
  const y1 = Math.max(upem * 0.5, ...ys, vertical ? target : 0) + margin
  const layout = preview?.assembly
  return (
    <div className={styles.preview}>
      <label className={styles.checkRow}>
        {t('math.previewSize')}
        <input
          type="range"
          min={0}
          max={upem * 8}
          step={1}
          value={Math.min(target, upem * 8)}
          aria-label={t('math.previewSize')}
          onChange={(event) => setTarget(Number(event.target.value))}
        />
        <output>{target}</output>
      </label>
      {error && <p role="alert">{error}</p>}
      <svg
        className={styles.canvas}
        viewBox={`${x0} ${-y1} ${x1 - x0} ${y1 - y0}`}
        role="img"
        aria-label={t('math.previewLabel', { glyph })}
      >
        <g transform="scale(1,-1)">
          {vertical ? (
            <path d={`M ${x0} ${target} H ${x1}`} className={styles.target} />
          ) : (
            <path d={`M ${target} ${y0} V ${y1}`} className={styles.target} />
          )}
          {placed.map((p, index) => (
            <path
              key={index}
              d={p.glyph.path}
              transform={`translate(${p.x} ${p.y})`}
              className={index % 2 ? styles.partAlt : styles.part}
            />
          ))}
        </g>
      </svg>
      <p aria-live="polite">
        {!preview
          ? t('glyphView.loading')
          : preview.variant
            ? t('math.previewVariant', { glyph: preview.variant })
            : layout
              ? t('math.previewAssembly', {
                  parts: layout.parts.length,
                  repeats: layout.repeats,
                  size: Math.round(layout.size),
                })
              : preview.fallback
                ? t('math.previewFallback', { glyph: preview.fallback })
                : t('math.previewEmpty')}
      </p>
      {layout?.short_connectors && (
        <p role="note">{t('math.shortConnectors')}</p>
      )}
      {layout?.limited && <p role="note">{t('math.partLimit')}</p>}
    </div>
  )
}
