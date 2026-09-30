import { useTranslation } from 'react-i18next'
import type { FontCompareResult } from '../../api'
import styles from './crossDocument.module.css'

function List({ label, items }: { label: string; items: readonly string[] }) {
  if (items.length === 0) return null
  return (
    <li>
      {label}: {items.join(', ')}
    </li>
  )
}

/** Font-level metadata, kerning and layout differences behind the glyph table. */
export function ComparisonSections({ result }: { result: FontCompareResult }) {
  const { t } = useTranslation()
  const { metadata, kerning, layout } = result
  const kerningQuiet =
    kerning.pairs_only_target === 0 &&
    kerning.pairs_only_source === 0 &&
    kerning.pairs_changed === 0 &&
    kerning.groups_only_target.length === 0 &&
    kerning.groups_only_source.length === 0 &&
    kerning.groups_changed.length === 0
  return (
    <>
      <section aria-label={t('crossDocument.metadataTitle')}>
        <h4>{t('crossDocument.metadataTitle')}</h4>
        {metadata.length === 0 ? (
          <p role="status">{t('crossDocument.metadataEqual')}</p>
        ) : (
          <div className={styles.scroll}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>{t('crossDocument.field')}</th>
                  <th>{t('crossDocument.targetValue')}</th>
                  <th>{t('crossDocument.sourceValue')}</th>
                </tr>
              </thead>
              <tbody>
                {metadata.map((row) => (
                  <tr key={row.field}>
                    <td>
                      <code>{row.field}</code>
                    </td>
                    <td>{row.target}</td>
                    <td>{row.source}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
      <section aria-label={t('crossDocument.kerningTitle')}>
        <h4>{t('crossDocument.kerningTitle')}</h4>
        {kerningQuiet ? (
          <p role="status">{t('crossDocument.kerningEqual')}</p>
        ) : (
          <>
            <p>
              {t('crossDocument.kerningCounts', {
                equal: kerning.pairs_equal,
                changed: kerning.pairs_changed,
                onlyTarget: kerning.pairs_only_target,
                onlySource: kerning.pairs_only_source,
              })}
            </p>
            <ul className={styles.help}>
              <List
                label={t('crossDocument.groupsOnlyTarget')}
                items={kerning.groups_only_target}
              />
              <List
                label={t('crossDocument.groupsOnlySource')}
                items={kerning.groups_only_source}
              />
              <List
                label={t('crossDocument.groupsChanged')}
                items={kerning.groups_changed}
              />
            </ul>
            {kerning.examples.length > 0 && (
              <table className={styles.table}>
                <thead>
                  <tr>
                    <th>{t('crossDocument.pair')}</th>
                    <th>{t('crossDocument.targetValue')}</th>
                    <th>{t('crossDocument.sourceValue')}</th>
                  </tr>
                </thead>
                <tbody>
                  {kerning.examples.map((pair, index) => (
                    <tr key={index}>
                      <td>
                        {pair.left} · {pair.right}
                        {pair.vertical
                          ? ` (${t('crossDocument.vertical')})`
                          : ''}
                      </td>
                      <td>{pair.target ?? '—'}</td>
                      <td>{pair.source ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}
      </section>
      <section aria-label={t('crossDocument.layoutTitle')}>
        <h4>{t('crossDocument.layoutTitle')}</h4>
        {result.compatibility.layout ? (
          <p role="status">{t('crossDocument.layoutEqual')}</p>
        ) : (
          <ul className={styles.help}>
            <List
              label={t('crossDocument.layoutOnlyTarget')}
              items={layout.only_target}
            />
            <List
              label={t('crossDocument.layoutOnlySource')}
              items={layout.only_source}
            />
            <List
              label={t('crossDocument.layoutChanged')}
              items={layout.changed}
            />
            <List
              label={t('crossDocument.classesOnlyTarget')}
              items={layout.classes_only_target}
            />
            <List
              label={t('crossDocument.classesOnlySource')}
              items={layout.classes_only_source}
            />
            <List
              label={t('crossDocument.classesChanged')}
              items={layout.classes_changed}
            />
            {layout.source_of_truth_changed && (
              <li>{t('crossDocument.layoutAuthority')}</li>
            )}
            {!layout.feature_source_equal && (
              <li>{t('crossDocument.layoutSource')}</li>
            )}
          </ul>
        )}
      </section>
    </>
  )
}
