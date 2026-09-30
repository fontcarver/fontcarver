import { useTranslation } from 'react-i18next'
import type { ExportFormat } from '../api'
import styles from './dialogs.module.css'

/** Output formats with a roving keyboard focus inside the tab list. */
export function ExportFormatTabs({
  formats,
  format,
  busy,
  onChange,
}: {
  formats: readonly ExportFormat[]
  format: ExportFormat
  busy: boolean
  onChange: (format: ExportFormat) => void
}) {
  const { t } = useTranslation()
  return (
    <div
      role="tablist"
      aria-label={t('dialog.export.format')}
      className={styles.tabs}
    >
      {formats.map((candidate) => (
        <button
          key={candidate}
          type="button"
          role="tab"
          disabled={busy}
          aria-selected={candidate === format}
          tabIndex={candidate === format ? 0 : -1}
          className={
            candidate === format
              ? `${styles.tab} ${styles.tabActive}`
              : styles.tab
          }
          onClick={() => onChange(candidate)}
          onKeyDown={(event) => {
            const delta =
              event.key === 'ArrowRight'
                ? 1
                : event.key === 'ArrowLeft'
                  ? -1
                  : 0
            if (!delta && event.key !== 'Home' && event.key !== 'End') return
            event.preventDefault()
            const index =
              event.key === 'Home'
                ? 0
                : event.key === 'End'
                  ? formats.length - 1
                  : (formats.indexOf(candidate) + delta + formats.length) %
                    formats.length
            const next = formats[index]
            if (next) onChange(next)
            const tabs =
              event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>(
                '[role="tab"]',
              )
            tabs?.[index]?.focus()
          }}
        >
          {t(`dialog.export.formatName.${candidate}`)}
        </button>
      ))}
    </div>
  )
}
