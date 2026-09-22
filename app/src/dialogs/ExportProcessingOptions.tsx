import { useTranslation } from 'react-i18next'

import type { ExportFormat } from '../api'
import styles from './dialogs.module.css'
import { SFNT_FORMATS, TYPE1_FORMATS } from './exportFormats'

const OPTION_KEYS = {
  remove_overlap: 'removeOverlap',
  autohint: 'autohint',
  keep_tt_instructions: 'keepTtInstructions',
  preserve_untouched_tables: 'preserveUntouchedTables',
  production_names: 'productionNames',
} as const

/** Export options that can be toggled without changing the source document. */
export type ExportOptionField = keyof typeof OPTION_KEYS

/** Shows only processing options supported by the selected output format. */
export function ExportProcessingOptions({
  format,
  canVary,
  variable,
  instance,
  busy,
  options,
  onChange,
}: {
  format: ExportFormat
  canVary: boolean
  variable: boolean
  instance: number | undefined
  busy: boolean
  options: Record<ExportOptionField, boolean>
  onChange: (field: ExportOptionField, value: boolean) => void
}) {
  const { t } = useTranslation()
  const fields: ExportOptionField[] = TYPE1_FORMATS.includes(format)
    ? ['remove_overlap', 'autohint']
    : SFNT_FORMATS.includes(format)
      ? (Object.keys(OPTION_KEYS) as ExportOptionField[]).filter(
          (field) => format !== 'Otf' || field !== 'keep_tt_instructions',
        )
      : []
  if (fields.length === 0) return null
  const unavailable = (field: ExportOptionField) =>
    canVary &&
    ((field === 'keep_tt_instructions' &&
      (variable || instance !== undefined)) ||
      (variable && (field === 'autohint' || field === 'remove_overlap')))

  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>{t('dialog.export.options')}</h3>
      {fields.map((field) => (
        <div key={field} className={styles.checkRow}>
          <input
            id={`export-${field}`}
            type="checkbox"
            disabled={busy || unavailable(field)}
            checked={!unavailable(field) && options[field]}
            onChange={(event) => onChange(field, event.target.checked)}
          />
          <label htmlFor={`export-${field}`} className={styles.checkLabel}>
            {t(`dialog.export.option.${OPTION_KEYS[field]}`)}
          </label>
          <span className={styles.checkHint}>
            {t(`dialog.export.option.${OPTION_KEYS[field]}Hint`)}
          </span>
        </div>
      ))}
    </div>
  )
}
