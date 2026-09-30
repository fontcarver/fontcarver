import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { FontCompareGlyphResult } from '../../api'
import styles from './crossDocument.module.css'

/** Two exact outlines share a baseline; optional normalization uses each em. */
export function ComparisonOutline({
  outline,
}: {
  outline: FontCompareGlyphResult
}) {
  const { t } = useTranslation()
  const [normalize, setNormalize] = useState(false)
  const targetUpem = outline.target?.upem ?? 0
  const sourceUpem = outline.source?.upem ?? 0
  const upem = targetUpem > 0 ? targetUpem : sourceUpem > 0 ? sourceUpem : 1000
  const ratio = normalize && sourceUpem > 0 ? upem / sourceUpem : 1
  const targetBounds = outline.target?.bounds ?? [0, 0, 0, 0]
  const sourceBounds = (outline.source?.bounds ?? [0, 0, 0, 0]).map(
    (value) => value * ratio,
  )
  const margin = upem * 0.08
  const x0 = Math.min(0, targetBounds[0] ?? 0, sourceBounds[0] ?? 0) - margin
  const y0 = Math.min(0, targetBounds[1] ?? 0, sourceBounds[1] ?? 0) - margin
  const x1 =
    Math.max(
      outline.target?.width ?? 0,
      (outline.source?.width ?? 0) * ratio,
      targetBounds[2] ?? 0,
      sourceBounds[2] ?? 0,
    ) + margin
  const y1 = Math.max(upem, targetBounds[3] ?? 0, sourceBounds[3] ?? 0) + margin
  return (
    <figure className={styles.figure}>
      <figcaption>{outline.name}</figcaption>
      <svg
        className={styles.canvas}
        role="img"
        aria-label={t('crossDocument.overlay', { name: outline.name })}
        viewBox={`${x0} ${-y1} ${x1 - x0} ${y1 - y0}`}
      >
        <g transform="scale(1,-1)">
          <path
            d={`M ${x0} 0 H ${x1} M 0 ${y0} V ${y1}`}
            className={styles.metrics}
            vectorEffect="non-scaling-stroke"
          />
          {outline.target && (
            <path
              d={outline.target.path}
              className={styles.target}
              vectorEffect="non-scaling-stroke"
            />
          )}
          {outline.source && (
            <path
              d={outline.source.path}
              transform={`scale(${ratio})`}
              className={styles.source}
              vectorEffect="non-scaling-stroke"
            />
          )}
        </g>
      </svg>
      <p className={styles.legend}>
        <span>{t('crossDocument.targetOutline')}</span>
        <span className={styles.legendSource}>
          {t('crossDocument.sourceOutline')}
        </span>
      </p>
      <p className={styles.help}>
        {t('crossDocument.widths', {
          target: outline.target?.width ?? '—',
          source: outline.source?.width ?? '—',
        })}
      </p>
      <label>
        <input
          type="checkbox"
          checked={normalize}
          onChange={(event) => setNormalize(event.target.checked)}
        />{' '}
        {t('crossDocument.normalizeEm')}
      </label>
      {(outline.target?.approximated || outline.source?.approximated) && (
        <p role="note">{t('crossDocument.approximated')}</p>
      )}
    </figure>
  )
}
