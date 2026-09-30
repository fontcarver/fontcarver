import { useTranslation } from 'react-i18next'
import type { Instance, InstanceOverrides } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import type { EditableAxis } from './designSpaceSession'
import { designDefault } from './designCoordinates'
import { overridesFamily, overridesStyle } from './scopedInfo'
import styles from './designSpace.module.css'

/** Export-scope numeric overrides and the font-level field each inherits. */
const NUMERIC_OVERRIDES = [
  ['weight_class', 'fontInfo.scoped.field.weightClass'],
  ['width_class', 'fontInfo.scoped.field.widthClass'],
] as const

/** Export-scope name overrides. */
const NAME_OVERRIDES = [
  ['postscript_font_name', 'fontInfo.scoped.field.postscriptFontName'],
  ['preferred_family_name', 'fontInfo.scoped.field.preferredFamily'],
  ['preferred_subfamily_name', 'fontInfo.scoped.field.preferredSubfamily'],
] as const

/** OpenType defaults used when neither the export nor the font states a class. */
const DEFAULT_NUMERIC = { weight_class: 400, width_class: 5 } as const

/** A named instance with explicit naming overrides and design coordinates. */
export function InstanceEditor({
  instance,
  familyName,
  axes,
  inherited = {},
  onChange,
  onRemove,
}: {
  instance: Instance
  familyName: string
  axes: EditableAxis[]
  /** Font-level values the export scope inherits when it overrides nothing. */
  inherited?: Partial<InstanceOverrides>
  onChange: (value: Instance) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  const overrides: Partial<InstanceOverrides> = instance.overrides ?? {}
  const setOverride = <K extends keyof InstanceOverrides>(
    field: K,
    value: InstanceOverrides[K] | undefined,
  ) => {
    const next = { ...overrides }
    if (value === undefined) delete next[field]
    else next[field] = value
    onChange({ ...instance, overrides: next })
  }
  return (
    <fieldset className={styles.card}>
      <legend>{instance.name}</legend>
      <div className={styles.row}>
        <label>
          {t('fontInfo.column.instanceName')}
          <input
            value={instance.name}
            onChange={(e) => onChange({ ...instance, name: e.target.value })}
          />
        </label>
        <label>
          {t('fontInfo.column.styleName')}
          <input
            value={instance.style_name}
            placeholder={instance.name}
            onChange={(e) =>
              onChange({ ...instance, style_name: e.target.value })
            }
          />
        </label>
        <label>
          {t('variation.familyOverride')}
          <input
            value={instance.family_name ?? ''}
            placeholder={familyName}
            onChange={(e) =>
              onChange({ ...instance, family_name: e.target.value || null })
            }
          />
        </label>
        <Button
          disabled={instance.family_name === null}
          onClick={() => onChange({ ...instance, family_name: null })}
        >
          {t('scopedInfo.useFontFamily')}
        </Button>
        <Button onClick={onRemove}>{t('variation.removeInstance')}</Button>
      </div>
      <div className={styles.row}>
        <span className={styles.sourceLabel}>
          {t(
            overridesFamily(instance.family_name, familyName)
              ? 'scopedInfo.exportOverride'
              : 'scopedInfo.inheritedFamily',
            { value: instance.family_name ?? familyName },
          )}
        </span>
        <span className={styles.sourceLabel}>
          {t(
            overridesStyle(instance.style_name, instance.name)
              ? 'scopedInfo.exportOverride'
              : 'scopedInfo.instanceStyle',
            {
              value: instance.style_name.trim()
                ? instance.style_name
                : instance.name,
            },
          )}
        </span>
      </div>
      <div className={styles.row}>
        {axes.map((axis) => (
          <NumberField
            key={axis.tag}
            label={axis.tag}
            value={instance.location[axis.tag] ?? designDefault(axis)}
            onCommit={(number) =>
              onChange({
                ...instance,
                location: { ...instance.location, [axis.tag]: number },
              })
            }
          />
        ))}
      </div>
      <div className={styles.row}>
        {(['active', 'is_variable_named_instance'] as const).map((field) => (
          <label key={field}>
            <input
              type="checkbox"
              checked={instance[field]}
              onChange={(e) =>
                onChange({ ...instance, [field]: e.target.checked })
              }
            />
            {t(
              field === 'active'
                ? 'fontInfo.column.active'
                : 'fontInfo.column.variableInstance',
            )}
          </label>
        ))}
      </div>
      <details>
        <summary>{t('scopedInfo.exportOverrides')}</summary>
        <p className={styles.sourceLabel}>
          {t('scopedInfo.exportOverridesHint')}
        </p>
        <div className={styles.row}>
          {NUMERIC_OVERRIDES.map(([field, label]) => (
            <div key={field} className={styles.row}>
              <label>
                <input
                  type="checkbox"
                  checked={overrides[field] !== undefined}
                  aria-label={t('scopedInfo.overrideField', {
                    field: t(label),
                  })}
                  onChange={(event) =>
                    setOverride(
                      field,
                      event.target.checked
                        ? (inherited[field] ?? DEFAULT_NUMERIC[field])
                        : undefined,
                    )
                  }
                />
                {t('variation.override')}
              </label>
              <NumberField
                label={t(label)}
                value={
                  overrides[field] ?? inherited[field] ?? DEFAULT_NUMERIC[field]
                }
                disabled={overrides[field] === undefined}
                onCommit={(value) => setOverride(field, Math.round(value))}
              />
              <span className={styles.sourceLabel}>
                {t(
                  overrides[field] === undefined
                    ? 'scopedInfo.inheritedFromFont'
                    : 'scopedInfo.exportOverride',
                  {
                    value:
                      overrides[field] ??
                      inherited[field] ??
                      DEFAULT_NUMERIC[field],
                  },
                )}
              </span>
            </div>
          ))}
        </div>
        <div className={styles.row}>
          {NAME_OVERRIDES.map(([field, label]) => (
            <label key={field}>
              {t(label)}
              <input
                value={overrides[field] ?? ''}
                placeholder={inherited[field] ?? ''}
                onChange={(event) =>
                  setOverride(field, event.target.value || undefined)
                }
              />
            </label>
          ))}
          <Button
            disabled={
              instance.overrides === undefined ||
              Object.values(instance.overrides).every(
                (value) => value === undefined || value === null,
              )
            }
            onClick={() => onChange({ ...instance, overrides: {} })}
          >
            {t('scopedInfo.revertOverrides')}
          </Button>
        </div>
      </details>
    </fieldset>
  )
}
