import { useTranslation } from 'react-i18next'

/** Common pixel grids whose cells align exactly with integer font units. */
export function integerGridPresets(unitsPerEm: number) {
  if (!Number.isInteger(unitsPerEm) || unitsPerEm <= 0) return []
  return [
    8, 10, 12, 16, 20, 24, 25, 32, 40, 48, 50, 64, 100, 125, 128, 200, 250, 256,
    512,
  ]
    .filter((ppem) => unitsPerEm % ppem === 0)
    .map((ppem) => ({ ppem, unitsPerPixel: unitsPerEm / ppem }))
}

/** Selects an exact pixel grid for the active font's units per em. */
export function BitmapGridPreset({
  unitsPerEm,
  value,
  onSelect,
  disabled = false,
}: {
  unitsPerEm: number
  value: number
  onSelect: (ppem: number) => void
  disabled?: boolean
}) {
  const { t } = useTranslation()
  const presets = integerGridPresets(unitsPerEm)
  return (
    <label>
      {t('bitmapGrid.title')}
      <select
        value={value}
        disabled={disabled || !presets.length}
        onChange={(event) => onSelect(Number(event.target.value))}
      >
        {!presets.some(({ ppem }) => ppem === value) && (
          <option value={value}>
            {t('bitmapGrid.custom', { ppem: value })}
          </option>
        )}
        {presets.map(({ ppem, unitsPerPixel }) => (
          <option key={ppem} value={ppem}>
            {t('bitmapGrid.preset', { ppem, unitsPerPixel })}
          </option>
        ))}
      </select>
    </label>
  )
}
