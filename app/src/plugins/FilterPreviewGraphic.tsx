import { useTranslation } from 'react-i18next'
import type { StyleBatchExample } from '../api'
import styles from './plugins.module.css'

/** Review of the original and proposed outline in font coordinates. */
export function FilterPreviewGraphic({
  example,
}: {
  example: StyleBatchExample
}) {
  const { t } = useTranslation()
  const [x0, y0, x1, y1] = example.bounds
  const padding = Math.max(10, (x1 - x0) * 0.05, (y1 - y0) * 0.05)
  return (
    <figure className={styles.figure}>
      <svg
        className={styles.preview}
        role="img"
        aria-label={t('plugins.previewOutline')}
        viewBox={`${x0 - padding} ${-y1 - padding} ${Math.max(1, x1 - x0) + padding * 2} ${Math.max(1, y1 - y0) + padding * 2}`}
      >
        <g transform="scale(1 -1)">
          <path
            className={styles.before}
            d={example.before}
            vectorEffect="non-scaling-stroke"
          />
          <path
            className={styles.after}
            d={example.after}
            vectorEffect="non-scaling-stroke"
          />
        </g>
      </svg>
      <figcaption>{t('plugins.previewLegend')}</figcaption>
    </figure>
  )
}
