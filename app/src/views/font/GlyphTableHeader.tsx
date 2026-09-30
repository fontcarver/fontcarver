import { useRef } from 'react'
import { ArrowUp, ChevronLeft, ChevronRight } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import type { GlyphSort } from '../../api'

import { glyphTableTemplate, type GlyphTableColumn } from './glyphTableColumns'
import styles from './glyphTableHeader.module.css'

const COLUMN_SORTS: Partial<Record<GlyphTableColumn, GlyphSort>> = {
  name: 'Name',
  unicode: 'Unicode',
  width: 'Width',
}

export interface GlyphTableHeaderProps {
  columns: readonly GlyphTableColumn[]
  sort: GlyphSort
  onSort(sort: GlyphSort): void
  /** Move a column immediately before another column. */
  onMove(column: GlyphTableColumn, beforeColumn: GlyphTableColumn): void
  className?: string
}

/** Sortable metadata headings with drag and keyboard column reordering. */
export function GlyphTableHeader({
  columns,
  sort,
  onSort,
  onMove,
  className,
}: GlyphTableHeaderProps) {
  const { t } = useTranslation()
  const draggedColumn = useRef<GlyphTableColumn | null>(null)

  return (
    <div
      role="row"
      className={[styles.header, className].filter(Boolean).join(' ')}
      style={{ gridTemplateColumns: glyphTableTemplate(columns) }}
      onClick={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
      onKeyDown={(event) => event.stopPropagation()}
      onKeyUp={(event) => event.stopPropagation()}
    >
      {columns.map((column, index) => {
        const label = t(`fontView.tableColumns.${column}`)
        const columnSort = COLUMN_SORTS[column]
        const sorted = columnSort !== undefined && sort === columnSort
        const previous = columns[index - 1]
        const next = columns[index + 1]
        return (
          <div
            key={column}
            role="columnheader"
            data-column={column}
            aria-sort={sorted ? 'ascending' : undefined}
            className={styles.column}
            draggable
            onDragStart={(event) => {
              event.stopPropagation()
              draggedColumn.current = column
              event.dataTransfer.effectAllowed = 'move'
              event.dataTransfer.setData(
                'application/x-fontcarver-glyph-table-column',
                column,
              )
            }}
            onDragEnd={() => {
              draggedColumn.current = null
            }}
            onDragOver={(event) => {
              if (draggedColumn.current === null) return
              event.preventDefault()
              event.stopPropagation()
              event.dataTransfer.dropEffect = 'move'
            }}
            onDrop={(event) => {
              const source = draggedColumn.current
              draggedColumn.current = null
              if (source === null) return
              event.preventDefault()
              event.stopPropagation()
              if (source !== column) onMove(source, column)
            }}
          >
            {columnSort === undefined ? (
              <span className={styles.label}>{label}</span>
            ) : (
              <button
                type="button"
                className={styles.sort}
                onClick={() => onSort(columnSort)}
              >
                <span className={styles.label}>{label}</span>
                {sorted && <ArrowUp size={12} aria-hidden="true" />}
              </button>
            )}
            <span className={styles.moves}>
              <button
                type="button"
                className={styles.move}
                aria-label={t('fontView.moveColumnLeft', { column: label })}
                disabled={previous === undefined}
                onClick={() =>
                  previous !== undefined && onMove(column, previous)
                }
              >
                <ChevronLeft size={12} aria-hidden="true" />
              </button>
              <button
                type="button"
                className={styles.move}
                aria-label={t('fontView.moveColumnRight', { column: label })}
                disabled={next === undefined}
                onClick={() => next !== undefined && onMove(next, column)}
              >
                <ChevronRight size={12} aria-hidden="true" />
              </button>
            </span>
          </div>
        )
      })}
    </div>
  )
}
