import { useTranslation } from 'react-i18next'
import type { KerningMode } from './kerningModes'
import styles from './TextBar.module.css'

/** Per-tab policy for kerning and protected spacing edits. */
export function KerningModeControl({
  mode,
  onChange,
}: {
  mode: KerningMode
  onChange: (mode: KerningMode) => void
}) {
  const { t } = useTranslation()
  return (
    <select
      className={styles.control}
      value={mode}
      onChange={(event) => onChange(event.currentTarget.value as KerningMode)}
      aria-label={t('kerningModes.label')}
      title={t(`kerningModes.${mode}Hint`)}
    >
      {(['off', 'on', 'locked'] as const).map((value) => (
        <option key={value} value={value}>
          {t(`kerningModes.${value}`)}
        </option>
      ))}
    </select>
  )
}
