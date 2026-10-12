import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type BitmapGridChoice,
  type BitmapGridInfo,
  type DocId,
} from '../../api'
import { reportError } from '../../stores/uiStore'
import styles from './BitmapView.module.css'

/**
 * Pixel grids whose cell is a whole number of font units.
 *
 * The list is the kernel's (`bitmap.grid`): every exact divisor of the font's
 * units per em from 4 pixels up to the em itself, not a curated set. A size
 * outside that set is still allowed — most bitmap fonts were never drawn on an
 * exact grid — but it is reported with the nearest exact size beside it, and
 * applying a choice carries the strike metrics the kernel derived from the
 * master.
 */
export function BitmapGridPreset({
  doc,
  value,
  master,
  onSelect,
  disabled = false,
}: {
  doc: DocId
  value: number
  master?: string
  onSelect: (choice: BitmapGridChoice) => void
  disabled?: boolean
}) {
  const { t } = useTranslation()
  const [info, setInfo] = useState<BitmapGridInfo | null>(null)
  useEffect(() => {
    let active = true
    void call('bitmap.grid', {
      doc,
      master: master ?? null,
      ppem: Number.isInteger(value) && value > 0 ? value : null,
    })
      .then((result) => {
        if (active) setInfo(result)
      })
      .catch(reportError)
    return () => {
      active = false
    }
  }, [doc, master, value])
  const presets = info?.presets ?? []
  const requested = info?.requested ?? null
  const nearest = info?.nearest ?? null
  const exactChoice = presets.find((preset) => preset.ppem === value)
  return (
    <div className={styles.grid}>
      <label>
        {t('bitmapGrid.title')}
        <select
          value={exactChoice ? value : ''}
          disabled={disabled || !presets.length}
          onChange={(event) => {
            const choice = presets.find(
              (preset) => preset.ppem === Number(event.target.value),
            )
            if (choice) onSelect(choice)
          }}
        >
          <option value="">
            {requested && !requested.exact
              ? t('bitmapGrid.custom', { ppem: value })
              : t('bitmapGrid.choose')}
          </option>
          {presets.map((preset) => (
            <option key={preset.ppem} value={preset.ppem}>
              {t('bitmapGrid.preset', {
                ppem: preset.ppem,
                unitsPerPixel: preset.units_per_pixel,
              })}
            </option>
          ))}
        </select>
      </label>
      {requested && !requested.exact && (
        <p role="status" className={styles.hint}>
          {t('bitmapGrid.inexact', {
            ppem: requested.ppem,
            unitsPerPixel: Number(requested.units_per_pixel.toFixed(3)),
          })}{' '}
          {nearest !== null && (
            <button
              type="button"
              disabled={disabled}
              onClick={() => {
                const choice = presets.find((preset) => preset.ppem === nearest)
                if (choice) onSelect(choice)
              }}
            >
              {t('bitmapGrid.useNearest', { ppem: nearest })}
            </button>
          )}
        </p>
      )}
      {requested?.exact && (
        <p className={styles.hint}>
          {t('bitmapGrid.exact', {
            ppem: requested.ppem,
            unitsPerPixel: Number(requested.units_per_pixel.toFixed(3)),
            ascent: requested.ascent,
            descent: requested.descent,
          })}
        </p>
      )}
    </div>
  )
}
