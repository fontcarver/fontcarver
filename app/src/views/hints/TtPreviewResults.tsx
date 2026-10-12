import { useTranslation } from 'react-i18next'
import type { TtHintPreview } from '../../api'
import { warningMessage } from '../../i18n/warnings'
import styles from './hints.module.css'

export function TtPreviewResults({ preview }: { preview: TtHintPreview }) {
  const { t } = useTranslation()
  return (
    <section aria-label={t('tt.previewResults')}>
      <h3>{t('tt.previewResults')}</h3>
      <p>{t('tt.previewLegend')}</p>
      <div className={styles.samples}>
        {preview.sizes.map((sample, index) => (
          <figure key={index}>
            <svg
              className={styles.preview}
              role="img"
              aria-label={t('tt.sample', { ppem: sample.ppem })}
              viewBox={`-2 ${-sample.ppem - 2} ${Math.max(sample.ppem, sample.hinted_advance, sample.unhinted_advance) + 4} ${sample.ppem * 1.3 + 4}`}
            >
              <g transform="scale(1,-1)">
                <path d={sample.unhinted} className={styles.unhinted} />
                <path d={sample.hinted} className={styles.hinted} />
                <path
                  d={`M ${sample.hinted_advance} ${-sample.ppem * 0.2} V ${sample.ppem}`}
                  className={styles.vertical}
                />
              </g>
            </svg>
            <figcaption>
              {t('tt.sample', { ppem: sample.ppem })}
              <br />
              {t('tt.advances', {
                hinted: sample.hinted_advance,
                unhinted: sample.unhinted_advance,
              })}
            </figcaption>
          </figure>
        ))}
      </div>
      {preview.warnings.map((warning, index) => (
        <p key={index}>{warningMessage(warning, t)}</p>
      ))}
    </section>
  )
}
