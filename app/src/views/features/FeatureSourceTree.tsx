import { useTranslation } from 'react-i18next'
import type { FeatureGenerationSettings, Features } from '../../api'
import { sourceItems, type SourceKind } from './featureSources'
import styles from './FeaturesView.module.css'

/** Source sections keep stable block identities even for repeated tags. */
export function FeatureSourceTree({
  source,
  settings,
  selected,
  readOnly,
  onSelect,
  onAdd,
}: {
  source: Features
  settings: FeatureGenerationSettings
  selected: string | null
  readOnly: boolean
  onSelect: (id: string) => void
  onAdd: (kind: SourceKind) => void
}) {
  const { t } = useTranslation()
  const generated = new Set(
    settings.generated.map((record) => record.source_id),
  )
  const items = sourceItems(source)
  return (
    <nav className={styles.tree} aria-label={t('features.sourceTree')}>
      {(['prefixes', 'classes', 'features', 'automatic'] as const).map(
        (kind) => {
          const entries = items.filter((item) =>
            kind === 'automatic'
              ? generated.has(item.id)
              : item.kind === kind && !generated.has(item.id),
          )
          return (
            <section key={kind} className={styles.treeSection}>
              <div className={styles.sectionHeader}>
                <h3>{t(`features.sections.${kind}`)}</h3>
                {kind !== 'automatic' && (
                  <button
                    type="button"
                    disabled={readOnly}
                    aria-label={t(`features.add.${kind}`)}
                    onClick={() => onAdd(kind)}
                  >
                    +
                  </button>
                )}
              </div>
              <ul>
                {entries.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      aria-current={item.id === selected ? 'true' : undefined}
                      className={styles.sourceItem}
                      onClick={() => onSelect(item.id)}
                    >
                      <span>
                        {item.kind === 'classes' ? '@' : ''}
                        {item.label || t('features.untitled')}
                      </span>
                      {item.disabled && (
                        <span className={styles.muted}>
                          {t('features.disabled')}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
              {entries.length === 0 && (
                <p className={styles.emptySection}>{t('features.noBlocks')}</p>
              )}
            </section>
          )
        },
      )}
    </nav>
  )
}
