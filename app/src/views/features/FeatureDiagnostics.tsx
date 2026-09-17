import { useTranslation } from 'react-i18next'
import type { FeatureValidationResult, Features } from '../../api'
import { sourceItems } from './featureSources'
import styles from './FeaturesView.module.css'

/** Located diagnostics use kernel-mapped UTF-16 positions, never byte offsets. */
export function FeatureDiagnostics({
  validation,
  source,
  onNavigate,
}: {
  validation: FeatureValidationResult | null
  source: Features
  onNavigate: (id: string, from: number, to: number) => void
}) {
  const { t } = useTranslation()
  const items = sourceItems(source)
  return (
    <section
      className={styles.diagnostics}
      aria-label={t('features.diagnostics')}
    >
      <h3>{t('features.diagnostics')}</h3>
      {!validation && (
        <p className={styles.muted}>{t('features.validationPending')}</p>
      )}
      {validation?.valid && (
        <p role="status">
          {t('features.validationPassed', {
            count: validation.compiled_masters.length,
          })}
        </p>
      )}
      {validation && (
        <ul>
          {validation.diagnostics.map((diagnostic, index) => {
            const item = items.find((item) => item.id === diagnostic.source_id)
            const from = diagnostic.span?.start ?? 0
            const to = diagnostic.span?.end ?? from
            return (
              <li
                key={`${diagnostic.code}:${index}`}
                className={
                  diagnostic.severity === 'Error'
                    ? styles.error
                    : styles.diagnostic
                }
              >
                <button
                  type="button"
                  disabled={!item}
                  onClick={() => item && onNavigate(item.id, from, to)}
                >
                  <span className={styles.diagnosticCode}>
                    {diagnostic.code}
                  </span>
                  <span>{diagnostic.message}</span>
                  {item && <span className={styles.muted}>{item.label}</span>}
                  {diagnostic.master && (
                    <span className={styles.muted}>{diagnostic.master}</span>
                  )}
                </button>
              </li>
            )
          })}
        </ul>
      )}
      {validation?.truncated && <p>{t('features.diagnosticsTruncated')}</p>}
    </section>
  )
}
