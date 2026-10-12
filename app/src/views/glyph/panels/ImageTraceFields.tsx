import { useTranslation } from 'react-i18next'

import type { ImageTraceSettings } from './useImageWorkflow'
import styles from './imagePanel.module.css'
import panel from './panels.module.css'

/** Whether every tracing option is inside the range the kernel accepts. */
export function validTraceSettings(settings: ImageTraceSettings): boolean {
  return (
    Number.isInteger(settings.threshold) &&
    settings.threshold >= 1 &&
    settings.threshold <= 255 &&
    Number.isFinite(settings.tolerance) &&
    settings.tolerance >= 0 &&
    settings.tolerance <= 16 &&
    Number.isFinite(settings.corner_angle) &&
    settings.corner_angle >= 0 &&
    settings.corner_angle <= 180 &&
    Number.isFinite(settings.smoothing) &&
    settings.smoothing >= 0 &&
    settings.smoothing <= 8 &&
    Number.isFinite(settings.min_area) &&
    settings.min_area >= 0 &&
    settings.min_area <= 1048576
  )
}

/** Autotrace options: thresholding, curve fitting, corners and speckle removal. */
export function ImageTraceFields({
  settings,
  onChange,
}: {
  settings: ImageTraceSettings
  onChange: (next: Partial<ImageTraceSettings>) => void
}) {
  const { t } = useTranslation()
  const number = (
    key: 'threshold' | 'tolerance' | 'corner_angle' | 'smoothing' | 'min_area',
    min: number,
    max: number,
    step: number,
  ) => (
    <label className={styles.field}>
      {t(`imageWorkflow.${key}`)}
      <input
        type="number"
        min={min}
        max={max}
        step={step}
        className={panel.textInput}
        value={Number.isFinite(settings[key]) ? settings[key] : ''}
        onChange={(event) => onChange({ [key]: event.target.valueAsNumber })}
      />
    </label>
  )
  return (
    <>
      <p className={styles.hint}>{t('imageWorkflow.traceHelp')}</p>
      {number('threshold', 1, 255, 1)}
      {number('tolerance', 0, 16, 0.1)}
      <label>
        <input
          type="checkbox"
          checked={settings.curves}
          onChange={(event) => onChange({ curves: event.target.checked })}
        />
        {t('imageWorkflow.curves')}
      </label>
      {settings.curves ? (
        <>
          {number('corner_angle', 0, 180, 1)}
          {number('smoothing', 0, 8, 0.5)}
          <p className={styles.hint}>{t('imageWorkflow.curveHelp')}</p>
        </>
      ) : (
        <p className={styles.hint}>{t('imageWorkflow.polygonHelp')}</p>
      )}
      {number('min_area', 0, 1048576, 1)}
      <label>
        <input
          type="checkbox"
          checked={settings.append}
          onChange={(event) => onChange({ append: event.target.checked })}
        />
        {t('imageWorkflow.append')}
      </label>
      {!settings.append && (
        <p className={styles.hint}>{t('imageWorkflow.replaceHelp')}</p>
      )}
    </>
  )
}
