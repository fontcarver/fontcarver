import { useTranslation } from 'react-i18next'
import type { FontMergeResult, MergeAdoptions } from '../../api'
import styles from './crossDocument.module.css'

/** Font-level adoption choices and the reviewed merge summary. */
export function MergeControls({
  adopt,
  disabled,
  onAdopt,
  merge,
  missingBases,
  onMissingBases,
}: {
  adopt: MergeAdoptions
  disabled: boolean
  onAdopt: (adopt: MergeAdoptions) => void
  merge: FontMergeResult | null
  missingBases: 'copy' | 'placeholder' | 'reject' | 'decompose'
  onMissingBases: (policy: 'copy' | 'placeholder' | 'reject') => void
}) {
  const { t } = useTranslation()
  return (
    <section aria-label={t('crossDocument.mergeTitle')} className={styles.form}>
      <h4>{t('crossDocument.mergeTitle')}</h4>
      <p className={styles.help}>{t('crossDocument.mergeHelp')}</p>
      <div className={styles.controls}>
        {(['metadata', 'master_metrics', 'kerning'] as const).map((key) => (
          <label key={key}>
            <input
              type="checkbox"
              disabled={disabled}
              checked={adopt[key]}
              onChange={(event) =>
                onAdopt({ ...adopt, [key]: event.target.checked })
              }
            />{' '}
            {t(`crossDocument.adopt.${key}`)}
          </label>
        ))}
      </div>
      <label className={styles.field}>
        {t('crossDocument.missingBases')}
        <select
          disabled={disabled}
          value={missingBases}
          onChange={(event) =>
            onMissingBases(
              event.target.value as 'copy' | 'placeholder' | 'reject',
            )
          }
        >
          {(['copy', 'placeholder', 'reject'] as const).map((value) => (
            <option key={value} value={value}>
              {t(`crossDocument.policies.${value}`)}
            </option>
          ))}
        </select>
      </label>
      {merge && (
        <div aria-live="polite">
          <p role="status">
            {t('crossDocument.mergeReview', {
              replaced: merge.replaced.length,
              kept: merge.kept.length,
              bases: merge.added_bases.length,
              label: merge.label,
            })}
          </p>
          {merge.replaced.length > 0 && (
            <p>
              {t('crossDocument.mergeReplaced', {
                names: merge.replaced.join(', '),
              })}
            </p>
          )}
          {merge.adopted.length > 0 && (
            <p>
              {t('crossDocument.mergeAdopted', {
                names: merge.adopted
                  .map((key) => t(`crossDocument.adopt.${key}`))
                  .join(', '),
              })}
            </p>
          )}
          {merge.warnings.map((warning, index) => (
            <p key={index} role="note">
              {warning}
            </p>
          ))}
        </div>
      )}
    </section>
  )
}
