import { useTranslation } from 'react-i18next'
import type { EditableAxis as Axis } from './designSpaceSession'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import styles from './designSpace.module.css'

/** Axis fields and explicit mapping rows; mappings are always user → design. */
export function AxisEditor({
  axis,
  onChange,
  onRemove,
}: {
  axis: Axis
  onChange: (axis: Axis) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  return (
    <fieldset className={styles.card}>
      <legend>{axis.name || axis.tag}</legend>
      <div className={styles.row}>
        <label>
          {t('fontInfo.column.tag')}
          <input
            aria-label={t('fontInfo.column.tag')}
            value={axis.tag}
            maxLength={4}
            onChange={(e) => onChange({ ...axis, tag: e.target.value })}
          />
        </label>
        <label>
          {t('fontInfo.column.axisName')}
          <input
            value={axis.name}
            onChange={(e) => onChange({ ...axis, name: e.target.value })}
          />
        </label>
        <Button onClick={onRemove}>{t('variation.removeAxis')}</Button>
      </div>
      <div className={styles.row}>
        {(['min', 'default', 'max'] as const).map((key) => (
          <NumberField
            key={key}
            label={t(`fontInfo.column.${key}`)}
            value={axis[key]}
            onCommit={(value) => onChange({ ...axis, [key]: value })}
          />
        ))}
        <label>
          <input
            type="checkbox"
            checked={axis.hidden}
            onChange={(e) => onChange({ ...axis, hidden: e.target.checked })}
          />
          {t('fontInfo.column.hidden')}
        </label>
      </div>
      <details>
        <summary>{t('variation.mapping')}</summary>
        <p>{t('variation.mappingHint')}</p>
        {axis.map.map(([user, design], index) => (
          <div className={styles.row} key={index}>
            <NumberField
              label={t('variation.userCoordinate')}
              value={user}
              onCommit={(value) =>
                onChange({
                  ...axis,
                  map: axis.map.map((pair, i) =>
                    i === index ? [value, pair[1]] : pair,
                  ),
                })
              }
            />
            <NumberField
              label={t('variation.designCoordinate')}
              value={design}
              onCommit={(value) =>
                onChange({
                  ...axis,
                  map: axis.map.map((pair, i) =>
                    i === index ? [pair[0], value] : pair,
                  ),
                })
              }
            />
            <Button
              onClick={() =>
                onChange({
                  ...axis,
                  map: axis.map.filter((_, i) => i !== index),
                })
              }
            >
              {t('variation.removeMapping')}
            </Button>
          </div>
        ))}
        <Button
          onClick={() =>
            onChange({
              ...axis,
              map: [
                ...axis.map,
                [
                  axis.map.length ? axis.max : axis.min,
                  axis.map.length ? axis.max : axis.min,
                ],
              ],
            })
          }
        >
          {t('variation.addMapping')}
        </Button>
      </details>
    </fieldset>
  )
}
