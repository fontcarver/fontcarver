import { useEffect, useRef, useState } from 'react'
import { Columns3 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { GLYPH_TABLE_COLUMNS, type GlyphTableColumn } from './glyphTableColumns'
import styles from './glyphTableHeader.module.css'

export interface GlyphTableColumnsMenuProps {
  /** Currently visible columns, in display order. */
  columns: readonly GlyphTableColumn[]
  /** Shows or hides one column. */
  onToggle(column: GlyphTableColumn): void
}

/**
 * Chooses which list columns appear.
 *
 * It sits beside the scrolling header rather than inside it, so the header's
 * grid tracks stay aligned with the virtualised rows. The last visible column
 * cannot be hidden: a table with no columns has nothing to read.
 */
export function GlyphTableColumnsMenu({
  columns,
  onToggle,
}: GlyphTableColumnsMenuProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const container = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const onPointerDown = (event: Event) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false)
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    document.addEventListener('pointerdown', onPointerDown, true)
    document.addEventListener('keydown', onKeyDown, true)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true)
      document.removeEventListener('keydown', onKeyDown, true)
    }
  }, [open])

  return (
    <div className={styles.picker} ref={container}>
      <button
        type="button"
        className={styles.move}
        aria-label={t('fontView.chooseColumns')}
        title={t('fontView.chooseColumns')}
        aria-expanded={open}
        onClick={() => setOpen((shown) => !shown)}
      >
        <Columns3 size={12} aria-hidden="true" />
      </button>
      {open && (
        <div
          className={styles.pickerMenu}
          role="group"
          aria-label={t('fontView.chooseColumns')}
        >
          {GLYPH_TABLE_COLUMNS.map((column) => {
            const shown = columns.includes(column)
            return (
              <label key={column} className={styles.pickerRow}>
                <input
                  type="checkbox"
                  checked={shown}
                  disabled={shown && columns.length === 1}
                  onChange={() => onToggle(column)}
                />
                {t(`fontView.tableColumns.${column}`)}
              </label>
            )
          })}
        </div>
      )}
    </div>
  )
}
