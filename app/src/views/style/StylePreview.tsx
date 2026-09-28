import { useTranslation } from 'react-i18next'
import type { StyleBatchExample } from '../../api'
import styles from './styleEffects.module.css'

/** Before and after share a coordinate frame, including their advance widths. */
export function StylePreview({
  example,
  upem,
  showSource = false,
}: {
  example: StyleBatchExample
  upem: number
  showSource?: boolean
}) {
  const { t } = useTranslation()
  const margin = upem * 0.08
  const [left, bottom, right, top] = example.bounds
  const x0 = Math.min(0, left) - margin
  const y0 = Math.min(0, bottom) - margin
  const x1 = Math.max(example.before_width, example.after_width, right) + margin
  const y1 = Math.max(showSource ? upem * 0.25 : upem, top) + margin
  const viewBox = `${x0} ${-y1} ${x1 - x0} ${y1 - y0}`
  return (
    <figure className={styles.example}>
      <figcaption>
        {example.glyph}
        {example.master && <> · {example.master}</>}
      </figcaption>
      <div className={styles.comparison}>
        {(['before', 'after'] as const).map((which) => (
          <div key={which}>
            <svg
              viewBox={viewBox}
              role="img"
              aria-label={t(`styleEffects.${which}`)}
              className={styles.canvas}
            >
              <g transform="scale(1,-1)">
                <path
                  d={example[which]}
                  className={styles.ink}
                  stroke={
                    showSource && which === 'before'
                      ? 'var(--color-text)'
                      : undefined
                  }
                  strokeWidth={1}
                  vectorEffect="non-scaling-stroke"
                />
                <path
                  d={`M ${x0} 0 H ${x1} M 0 ${y0} V ${y1} M ${example[`${which}_width`]} ${y0} V ${y1}`}
                  className={styles.metrics}
                />
              </g>
            </svg>
            <p>
              {t(`styleEffects.${which}`)} ·{' '}
              {t('styleEffects.advanceValue', {
                value: Number(example[`${which}_width`].toFixed(2)),
              })}
            </p>
          </div>
        ))}
      </div>
    </figure>
  )
}
