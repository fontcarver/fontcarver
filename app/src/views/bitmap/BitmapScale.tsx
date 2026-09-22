import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { BitmapGlyph } from '../../api'
import { reportError } from '../../stores/uiStore'
import { resample } from './bitmapTransforms'
import styles from './BitmapView.module.css'
export function BitmapScale({
  value,
  disabled,
  onEdit,
}: {
  value: BitmapGlyph
  disabled: boolean
  onEdit: (glyph: BitmapGlyph) => void
}) {
  const { t } = useTranslation(),
    [width, setWidth] = useState(value.bitmap.width),
    [height, setHeight] = useState(value.bitmap.height),
    [smooth, setSmooth] = useState(false)
  return (
    <details>
      <summary>{t('bitmap.scale')}</summary>
      <form
        className={styles.toolbar}
        onSubmit={(e) => {
          e.preventDefault()
          try {
            onEdit(resample(value, width, height, smooth))
          } catch (error) {
            reportError(error)
          }
        }}
      >
        <label>
          {t('bitmap.scaleWidth')}
          <input
            type="number"
            min={1}
            max={4096}
            required
            value={width}
            onChange={(e) => setWidth(Number(e.target.value))}
          />
        </label>
        <label>
          {t('bitmap.scaleHeight')}
          <input
            type="number"
            min={1}
            max={4096}
            required
            value={height}
            onChange={(e) => setHeight(Number(e.target.value))}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={smooth}
            onChange={(e) => setSmooth(e.target.checked)}
          />
          {t('bitmap.smooth')}
        </label>
        <button disabled={disabled}>{t('bitmap.scale')}</button>
        <p className={styles.hint}>{t('bitmap.scaleHint')}</p>
      </form>
    </details>
  )
}
