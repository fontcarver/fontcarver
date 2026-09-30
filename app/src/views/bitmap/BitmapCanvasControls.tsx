import { useTranslation } from 'react-i18next'
import type { PixelPoint } from './pixels'
import styles from './BitmapCanvas.module.css'

/** Visibility controls and the pixel-space pointer status. */
export function BitmapCanvasControls({
  grid,
  onGrid,
  outline,
  onOutline,
  hasOutline,
  unitsPerPixel,
  pointer,
  delta,
  advance,
}: {
  grid: boolean
  onGrid: (value: boolean) => void
  outline: boolean
  onOutline: (value: boolean) => void
  hasOutline: boolean
  unitsPerPixel?: number
  pointer: PixelPoint | null
  delta: PixelPoint | null
  advance: number
}) {
  const { t } = useTranslation()
  return (
    <div className={styles.controls}>
      <label>
        <input
          type="checkbox"
          checked={grid}
          onChange={(event) => onGrid(event.target.checked)}
        />
        {t('bitmapCanvas.grid')}
      </label>
      <label>
        <input
          type="checkbox"
          checked={outline}
          disabled={!hasOutline}
          onChange={(event) => onOutline(event.target.checked)}
        />
        {t('bitmapCanvas.outline')}
      </label>
      <span>{t('bitmapCanvas.advance', { value: advance })}</span>
      {unitsPerPixel !== undefined && (
        <span>
          {t('bitmapCanvas.unitsPerPixel', {
            value: Number(unitsPerPixel.toFixed(3)),
          })}
        </span>
      )}
      <output className={styles.pointer} aria-label={t('bitmapCanvas.pointer')}>
        {pointer
          ? t('bitmapCanvas.position', { x: pointer[0], y: pointer[1] })
          : t('bitmapCanvas.positionEmpty')}
        {delta && (
          <> · {t('bitmapCanvas.delta', { x: delta[0], y: delta[1] })}</>
        )}
      </output>
    </div>
  )
}
