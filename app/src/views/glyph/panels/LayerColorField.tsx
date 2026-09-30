import { useTranslation } from 'react-i18next'

import type { Layer } from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'
import { COLOR_LABELS, colorLabelKey } from '../../font/colorLabels'
import { commitField } from '../commitField'

import styles from './panels.module.css'

/** Stored independent layer label, rejecting malformed imported metadata. */
export function layerColor(layer: Layer): number | null {
  const data = layer.user_data
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null
  const color = (data as Record<string, unknown>)['com.fontcarver.colorLabel']
  return typeof color === 'number' &&
    Number.isInteger(color) &&
    color >= 0 &&
    color < 12
    ? color
    : null
}

/** Independent layer color; native history persists it without changing glyph labels. */
export function LayerColorField({
  target,
  layer,
  reload,
}: {
  target: LayerTarget
  layer: Layer
  reload: () => void
}) {
  const { t } = useTranslation()
  const value = layerColor(layer)
  return (
    <label className={styles.row}>
      <span className={styles.rowLabel}>{t('glyphMetadata.layerColor')}</span>
      <select
        className={styles.textInput}
        value={value ?? ''}
        onChange={(event) =>
          commitField(
            target,
            {
              SetLayerColor: {
                glyph: target.glyph,
                layer: layer.id,
                color:
                  event.currentTarget.value === ''
                    ? null
                    : Number(event.currentTarget.value),
              },
            },
            t('glyphMetadata.setLayerColor'),
            reload,
          )
        }
      >
        <option value="">{t('panel.glyph.noColor')}</option>
        {COLOR_LABELS.map((label) => (
          <option key={label.index} value={label.index}>
            {t(`fontView.color.${colorLabelKey(label.index)}`)}
          </option>
        ))}
      </select>
    </label>
  )
}
