import { useTranslation } from 'react-i18next'
import type { FontCompareFamilyResult } from '../../api'
import styles from './crossDocument.module.css'

/** Every (document, master) member of a family against the reference member. */
export function FamilyResults({
  family,
  labelFor,
}: {
  family: FontCompareFamilyResult
  labelFor: (doc: number) => string
}) {
  const { t } = useTranslation()
  return (
    <section aria-label={t('crossDocument.familyResults')}>
      <p className={styles.help}>{t('crossDocument.familyHelp')}</p>
      <p role="status">
        {t(
          family.compatible
            ? 'crossDocument.familyCompatible'
            : 'crossDocument.familyMismatched',
        )}
      </p>
      <table className={styles.table}>
        <thead>
          <tr>
            <th>{t('crossDocument.member')}</th>
            <th>{t('crossDocument.compatibilityTitle')}</th>
            <th>{t('crossDocument.differences')}</th>
          </tr>
        </thead>
        <tbody>
          {family.members.map((member) => {
            const failures = Object.entries(member.compatibility)
              .filter(([, matches]) => !matches)
              .map(([key]) => t(`crossDocument.compatibility.${key}`))
            return (
              <tr key={`${member.doc}:${member.master}`}>
                <td>
                  {labelFor(member.doc)} · {member.master_name}
                  {member.reference
                    ? ` · ${t('crossDocument.referenceMember')}`
                    : ''}
                </td>
                <td>
                  {member.issue
                    ? member.issue
                    : member.reference
                      ? t('crossDocument.referenceMember')
                      : failures.length === 0
                        ? t('crossDocument.compatible')
                        : failures.join(', ')}
                </td>
                <td>
                  {member.reference
                    ? '—'
                    : t('crossDocument.diffCounts', {
                        equal: member.counts.equal,
                        outlines: member.counts.outlines,
                        metrics: member.counts.metrics,
                        missing:
                          member.counts.missing_target +
                          member.counts.missing_source,
                        unicode: member.counts.unicodes,
                        components: member.counts.components,
                      })}
                  {member.examples.length > 0 && (
                    <span className={styles.help}>
                      {' '}
                      {member.examples.slice(0, 8).join(', ')}
                    </span>
                  )}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </section>
  )
}
