import { useId } from 'react'
import { useTranslation } from 'react-i18next'

import type { MetricSide } from './textMetrics'
import styles from './SpacingControls.module.css'

/** Controlled side selection for text-view spacing shortcuts. */
export interface SpacingControlsProps {
  side: MetricSide
  onSideChange: (side: MetricSide) => void
  disabled?: boolean
}

/** Selects the side used by bracket shortcuts and explains their step modes. */
export function SpacingControls({
  side,
  onSideChange,
  disabled = false,
}: SpacingControlsProps) {
  const { t } = useTranslation()
  const helpId = useId()
  const mod = /Mac|iPhone|iPad/u.test(navigator.platform) ? '⌘' : 'Ctrl'
  const help = t('spacingControls.help', { mod })

  return (
    <div className={styles.controls}>
      <select
        className={styles.side}
        value={side}
        disabled={disabled}
        aria-label={t('spacingControls.side')}
        aria-describedby={helpId}
        title={help}
        onChange={(event) =>
          onSideChange(event.currentTarget.value as MetricSide)
        }
      >
        <option value="lsb">{t('spacingControls.lsb')}</option>
        <option value="rsb">{t('spacingControls.rsb')}</option>
        <option value="both">{t('spacingControls.both')}</option>
      </select>
      <span className={styles.keys} aria-hidden="true" title={help}>
        {t('spacingControls.keys')}
      </span>
      <span id={helpId} className={styles.srOnly}>
        {help}
      </span>
    </div>
  )
}
