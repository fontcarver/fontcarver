import { useTranslation } from 'react-i18next'
import type { FeatureGenerationSettings, FeatureGenerator } from '../../api'
import styles from './FeaturesView.module.css'

/** Explicit automatic recipes; toggling never changes active layout by itself. */
export function FeatureGenerationPanel({
  settings,
  readOnly,
  canFreeze,
  stale,
  onChange,
  onFreeze,
}: {
  settings: FeatureGenerationSettings
  readOnly: boolean
  canFreeze: boolean
  stale: boolean
  onChange: (settings: FeatureGenerationSettings) => void
  onFreeze: (generator: FeatureGenerator) => void
}) {
  const { t } = useTranslation()
  const recipes = [
    'small_caps',
    'stylistic_sets',
    'case_forms',
    'vertical_forms',
    'ligatures',
  ] as const
  const generators = [
    ...new Set(settings.generated.map((record) => record.generator)),
  ]
  return (
    <aside className={styles.generation} aria-label={t('features.generation')}>
      <h3>{t('features.generation')}</h3>
      <p className={styles.muted}>{t('features.generationHelp')}</p>
      {stale && (
        <p role="status" className={styles.warning}>
          {t('features.generationStale')}
        </p>
      )}
      <fieldset disabled={readOnly} className={styles.checks}>
        {recipes.map((key) => (
          <label key={key}>
            <input
              type="checkbox"
              checked={settings[key]}
              onChange={(event) =>
                onChange({ ...settings, [key]: event.target.checked })
              }
            />
            {t(`features.recipes.${key}`)}
          </label>
        ))}
        <label>
          <input
            type="checkbox"
            checked={settings.writer_policy.kerning}
            onChange={(event) =>
              onChange({
                ...settings,
                writer_policy: {
                  ...settings.writer_policy,
                  kerning: event.target.checked,
                },
              })
            }
          />
          {t('features.writerKerning')}
        </label>
        <label>
          <input
            type="checkbox"
            checked={settings.writer_policy.anchors}
            onChange={(event) =>
              onChange({
                ...settings,
                writer_policy: {
                  ...settings.writer_policy,
                  anchors: event.target.checked,
                },
              })
            }
          />
          {t('features.writerAnchors')}
        </label>
      </fieldset>
      {generators.map((generator) => (
        <div key={generator} className={styles.generatedRecipe}>
          <span>{t(`features.generator.${generator}`)}</span>
          <button
            type="button"
            disabled={readOnly || !canFreeze}
            title={t(
              canFreeze ? 'features.freezeHelp' : 'features.freezePreview',
            )}
            onClick={() => onFreeze(generator)}
          >
            {t('features.freeze')}
          </button>
        </div>
      ))}
      <details>
        <summary>{t('features.localized')}</summary>
        <fieldset disabled={readOnly} className={styles.mappings}>
          {settings.localized.map((mapping, index) => (
            <div key={index} className={styles.mapping}>
              <label>
                {t('features.script')}
                <input
                  value={mapping.script}
                  maxLength={4}
                  onChange={(event) =>
                    onChange({
                      ...settings,
                      localized: settings.localized.map((item, i) =>
                        i === index
                          ? { ...item, script: event.target.value }
                          : item,
                      ),
                    })
                  }
                />
              </label>
              <label>
                {t('features.language')}
                <input
                  value={mapping.language}
                  maxLength={4}
                  onChange={(event) =>
                    onChange({
                      ...settings,
                      localized: settings.localized.map((item, i) =>
                        i === index
                          ? { ...item, language: event.target.value }
                          : item,
                      ),
                    })
                  }
                />
              </label>
              {mapping.substitutions.map((substitution, subIndex) => (
                <div key={subIndex} className={styles.substitution}>
                  {(['from', 'to'] as const).map((side) => (
                    <label key={side}>
                      {t(
                        side === 'from'
                          ? 'features.fromGlyph'
                          : 'features.toGlyph',
                      )}
                      <input
                        value={substitution[side]}
                        onChange={(event) =>
                          onChange({
                            ...settings,
                            localized: settings.localized.map((item, i) =>
                              i === index
                                ? {
                                    ...item,
                                    substitutions: item.substitutions.map(
                                      (sub, j) =>
                                        j === subIndex
                                          ? {
                                              ...sub,
                                              [side]: event.target.value,
                                            }
                                          : sub,
                                    ),
                                  }
                                : item,
                            ),
                          })
                        }
                      />
                    </label>
                  ))}
                </div>
              ))}
              <button
                type="button"
                onClick={() =>
                  onChange({
                    ...settings,
                    localized: settings.localized.filter((_, i) => i !== index),
                  })
                }
              >
                {t('features.removeMapping')}
              </button>
            </div>
          ))}
          <button
            type="button"
            onClick={() =>
              onChange({
                ...settings,
                localized: [
                  ...settings.localized,
                  {
                    script: 'latn',
                    language: 'dflt',
                    substitutions: [{ from: '', to: '' }],
                  },
                ],
              })
            }
          >
            {t('features.addMapping')}
          </button>
        </fieldset>
      </details>
      <details>
        <summary>{t('features.discretionary')}</summary>
        <fieldset disabled={readOnly} className={styles.mappings}>
          {settings.discretionary_ligatures.map((mapping, index) => (
            <div key={index} className={styles.mapping}>
              {(['script', 'language'] as const).map((field) => (
                <label key={field}>
                  {t(`features.${field}`)}
                  <input
                    value={mapping[field]}
                    maxLength={4}
                    onChange={(event) =>
                      onChange({
                        ...settings,
                        discretionary_ligatures:
                          settings.discretionary_ligatures.map((item, i) =>
                            i === index
                              ? { ...item, [field]: event.target.value }
                              : item,
                          ),
                      })
                    }
                  />
                </label>
              ))}
              <label>
                {t('features.components')}
                <input
                  value={mapping.components.join(' ')}
                  onChange={(event) =>
                    onChange({
                      ...settings,
                      discretionary_ligatures:
                        settings.discretionary_ligatures.map((item, i) =>
                          i === index
                            ? {
                                ...item,
                                components: event.target.value.split(' '),
                              }
                            : item,
                        ),
                    })
                  }
                />
              </label>
              <label>
                {t('features.toGlyph')}
                <input
                  value={mapping.replacement}
                  onChange={(event) =>
                    onChange({
                      ...settings,
                      discretionary_ligatures:
                        settings.discretionary_ligatures.map((item, i) =>
                          i === index
                            ? { ...item, replacement: event.target.value }
                            : item,
                        ),
                    })
                  }
                />
              </label>
              <button
                type="button"
                onClick={() =>
                  onChange({
                    ...settings,
                    discretionary_ligatures:
                      settings.discretionary_ligatures.filter(
                        (_, i) => i !== index,
                      ),
                  })
                }
              >
                {t('features.removeMapping')}
              </button>
            </div>
          ))}
          <button
            type="button"
            onClick={() =>
              onChange({
                ...settings,
                discretionary_ligatures: [
                  ...settings.discretionary_ligatures,
                  {
                    components: [],
                    replacement: '',
                    script: 'DFLT',
                    language: 'dflt',
                  },
                ],
              })
            }
          >
            {t('features.addMapping')}
          </button>
        </fieldset>
      </details>
    </aside>
  )
}
