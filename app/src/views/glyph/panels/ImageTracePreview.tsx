import { useTranslation } from 'react-i18next'
import type { ImageTraceResult } from '../../../api'
import styles from './imagePanel.module.css'

/** Trace preview with the quality a review needs before applying it. */
export function ImageTracePreview({ result }: { result: ImageTraceResult }) {
  const { t } = useTranslation()
  const [left, bottom, right, top] = result.bounds
  const width = Math.max(1, right - left)
  const height = Math.max(1, top - bottom)
  const pad = Math.max(width, height) * 0.05
  return (
    <>
      <svg
        className={styles.preview}
        role="img"
        aria-label={t('imageWorkflow.tracedOutlines')}
        viewBox={`${left - pad} ${-top - pad} ${width + pad * 2} ${height + pad * 2}`}
      >
        <path
          d={result.path}
          transform="scale(1 -1)"
          fill="currentColor"
          fillRule="nonzero"
        />
      </svg>
      <p className={styles.hint} role="status">
        {t('imageWorkflow.reviewed', { count: result.contours.length })}
      </p>
      <p className={styles.hint}>
        {t('imageWorkflow.quality', {
          nodes: result.nodes,
          curves: result.curve_segments,
          error: Math.round(result.max_error * 100) / 100,
        })}
      </p>
      {result.dropped > 0 && (
        <p className={styles.hint}>
          {t('imageWorkflow.droppedRings', { count: result.dropped })}
        </p>
      )}
      {result.fallback && (
        <p role="alert" className={styles.error}>
          {t('imageWorkflow.fallback')}
        </p>
      )}
    </>
  )
}
