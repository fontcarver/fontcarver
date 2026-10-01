import { useTranslation } from 'react-i18next'
import type { BitmapOverlays } from './bitmapToolStore'
import type { PixelPoint } from './pixels'
import styles from './BitmapCanvas.module.css'

/** Round a font-unit value for display without inventing precision. */
function units(pixels: number, unitsPerPixel: number): number {
  return Number((pixels * unitsPerPixel).toFixed(2))
}

/** Overlay switches and the pointer status in pixels and font units. */
export function BitmapCanvasControls({
  overlays,
  onOverlay,
  hasOutline,
  unitsPerPixel,
  unitsPerPixelLabel,
  pointer,
  delta,
  advance,
}: {
  overlays: BitmapOverlays
  onOverlay: (key: keyof BitmapOverlays, value: boolean) => void
  hasOutline: boolean
  /** Font units per pixel for the readouts, absent while they are switched off. */
  unitsPerPixel?: number
  /** Font units per pixel for the standing readout, independent of that switch. */
  unitsPerPixelLabel?: number
  pointer: PixelPoint | null
  delta: PixelPoint | null
  advance: number
}) {
  const { t } = useTranslation()
  const switches: {
    key: keyof BitmapOverlays
    label: string
    disabled?: boolean
  }[] = [
    { key: 'grid', label: t('bitmapCanvas.grid') },
    { key: 'outline', label: t('bitmapCanvas.outline'), disabled: !hasOutline },
    { key: 'advanceColumn', label: t('bitmapCanvas.advanceColumn') },
    { key: 'fontUnits', label: t('bitmapCanvas.fontUnits') },
  ]
  return (
    <div className={styles.controls}>
      {switches.map(({ key, label, disabled }) => (
        <label key={key}>
          <input
            type="checkbox"
            checked={overlays[key]}
            disabled={disabled}
            onChange={(event) => onOverlay(key, event.target.checked)}
          />
          {label}
        </label>
      ))}
      <span>
        {t('bitmapCanvas.advance', { value: advance })}
        {unitsPerPixel !== undefined &&
          ` (${t('bitmapCanvas.unitsValue', {
            value: units(advance, unitsPerPixel),
          })})`}
      </span>
      {unitsPerPixelLabel !== undefined && (
        <span>
          {t('bitmapCanvas.unitsPerPixel', {
            value: Number(unitsPerPixelLabel.toFixed(3)),
          })}
        </span>
      )}
      <output className={styles.pointer} aria-label={t('bitmapCanvas.pointer')}>
        {pointer
          ? t('bitmapCanvas.position', { x: pointer[0], y: pointer[1] })
          : t('bitmapCanvas.positionEmpty')}
        {pointer && unitsPerPixel !== undefined && (
          <>
            {' '}
            {t('bitmapCanvas.positionUnits', {
              x: units(pointer[0], unitsPerPixel),
              y: units(pointer[1], unitsPerPixel),
            })}
          </>
        )}
        {delta && (
          <> · {t('bitmapCanvas.delta', { x: delta[0], y: delta[1] })}</>
        )}
        {delta && unitsPerPixel !== undefined && (
          <>
            {' '}
            {t('bitmapCanvas.deltaUnits', {
              x: units(delta[0], unitsPerPixel),
              y: units(delta[1], unitsPerPixel),
            })}
          </>
        )}
      </output>
    </div>
  )
}
