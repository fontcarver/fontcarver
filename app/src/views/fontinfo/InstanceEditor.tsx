import { useTranslation } from 'react-i18next'
import type { Instance } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import type { EditableAxis } from './designSpaceSession'
import { designDefault } from './designCoordinates'
import styles from './designSpace.module.css'

/** A named instance with explicit naming overrides and design coordinates. */
export function InstanceEditor({
  instance,
  familyName,
  axes,
  onChange,
  onRemove,
}: {
  instance: Instance
  familyName: string
  axes: EditableAxis[]
  onChange: (value: Instance) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
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
            instance.family_name === null
              ? 'scopedInfo.inheritedFamily'
              : 'scopedInfo.exportOverride',
            { value: instance.family_name ?? familyName },
          )}
        </span>
        <span className={styles.sourceLabel}>
          {t(
            instance.style_name.trim()
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
    </fieldset>
  )
}
