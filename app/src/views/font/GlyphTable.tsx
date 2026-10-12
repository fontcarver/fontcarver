import {
  useEffect,
  useRef,
  useState,
  type MouseEvent,
  type PointerEvent,
} from 'react'
import { useTranslation } from 'react-i18next'

import type { GlyphCell } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'

import { formatUnicode } from './cellRenderer'
import { useFontViewPrefs } from './fontViewPrefs'
import { GlyphContextMenu } from './GlyphContextMenu'
import type { GlyphGridProps } from './GlyphGrid'
import { GlyphTableColumnsMenu } from './GlyphTableColumnsMenu'
import { GlyphTableHeader } from './GlyphTableHeader'
import { glyphTableTemplate, type GlyphTableColumn } from './glyphTableColumns'
import { glyphStatuses } from './glyphStatuses'
import styles from './glyphTable.module.css'
import { useGlyphSelection } from './useGlyphSelection'
import { useGridKeys } from './useGridKeys'
import { useGridPager } from './useGridPager'
import { indexAt, type MenuPosition } from './useGridPointer'
import { useScrollMemory } from './useScrollMemory'
import {
  GLYPH_TABLE_ROW_HEIGHT,
  useVirtualGlyphTable,
} from './useVirtualGlyphTable'

function columnValue(cell: GlyphCell, column: GlyphTableColumn): string {
  switch (column) {
    case 'name':
      return cell.name
    case 'unicode':
      return cell.unicodes.map(formatUnicode).join(', ') || '—'
    case 'width':
      return String(cell.width)
    case 'category':
      return cell.category
    case 'subCategory':
      return cell.sub_category
    case 'script':
      return cell.script
    case 'status':
      return ''
  }
}

/** Paged glyph rows sharing filters, selection and commands with the grid. */
export function GlyphTable({ summary, query }: GlyphGridProps) {
  const { t } = useTranslation()
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const setFocus = useDocumentStore((state) => state.setFocus)
  const containerRef = useRef<HTMLDivElement>(null)
  const headerRef = useRef<HTMLDivElement>(null)
  const [hasFocus, setHasFocus] = useState(false)
  const [menu, setMenu] = useState<MenuPosition | null>(null)
  const { pager, version, total } = useGridPager({ doc, query })
  const { metrics, scrollTop, scrollTo, onScroll } = useVirtualGlyphTable(
    total,
    containerRef,
  )
  useScrollMemory({
    query,
    containerRef,
    scrollTop,
    loaded: pager.total !== null,
    onScroll,
    presentation: 'list',
  })
  const selection = useGlyphSelection({ doc, query, pager, total, scrollTo })
  const { openFocused, onKeyDown } = useGridKeys({
    doc,
    pager,
    metrics,
    total,
    selection,
  })
  const columns = useFontViewPrefs((state) => state.tableColumns)
  const moveColumn = useFontViewPrefs((state) => state.moveColumn)
  const toggleColumn = useFontViewPrefs((state) => state.toggleColumn)
  const template = glyphTableTemplate(columns)

  useEffect(() => {
    pager.setRange(metrics.first, Math.max(metrics.first, metrics.last))
  }, [metrics.first, metrics.last, pager, version])

  useEffect(() => {
    setFocus(doc, pager.cell(selection.focusIndex)?.gid ?? null)
  }, [doc, pager, selection.focusIndex, setFocus, version])

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return
    setMenu(null)
    containerRef.current?.focus()
    const index = indexAt(event.target)
    if (index === null) {
      if (!event.shiftKey && !event.ctrlKey && !event.metaKey) selection.clear()
      return
    }
    selection.selectAt(index, {
      shift: event.shiftKey,
      additive: event.ctrlKey || event.metaKey,
    })
  }

  const onContextMenu = (event: MouseEvent<HTMLDivElement>) => {
    const index = indexAt(event.target)
    if (index === null) return
    event.preventDefault()
    const gid = pager.cell(index)?.gid
    if (gid !== undefined && view.selection.has(gid)) selection.anchorAt(index)
    else selection.selectAt(index, { shift: false, additive: false })
    setMenu({ x: event.clientX, y: event.clientY })
  }

  const rows = []
  for (let index = metrics.first; index <= metrics.last; index += 1) {
    const cell = pager.cell(index)
    if (!cell) continue
    const statuses = glyphStatuses(cell)
      .map((status) => t(status.labelKey))
      .join(', ')
    rows.push(
      <div
        key={cell.gid}
        role="row"
        aria-label={cell.name}
        aria-rowindex={index + 2}
        aria-selected={view.selection.has(cell.gid)}
        data-index={index}
        data-gid={cell.gid}
        data-glyph={cell.name}
        data-focused={hasFocus && selection.focusIndex === index}
        className={styles.row}
        style={{
          transform: `translateY(${index * GLYPH_TABLE_ROW_HEIGHT}px)`,
          gridTemplateColumns: template,
        }}
      >
        {columns.map((column) => (
          <div
            key={column}
            role="gridcell"
            className={styles.cell}
            title={column === 'status' ? statuses : columnValue(cell, column)}
          >
            {column === 'status' ? statuses : columnValue(cell, column)}
          </div>
        ))}
      </div>,
    )
  }

  return (
    <div
      role="grid"
      aria-label={t('fontView.tableLabel')}
      aria-rowcount={total + 1}
      aria-colcount={columns.length}
      aria-multiselectable="true"
      className={styles.table}
    >
      <div className={styles.headerBar}>
        <div ref={headerRef} className={styles.headerViewport}>
          <GlyphTableHeader
            columns={columns}
            sort={view.sort}
            onSort={(sort) => useDocumentStore.getState().setSort(doc, sort)}
            onMove={moveColumn}
          />
        </div>
        <GlyphTableColumnsMenu columns={columns} onToggle={toggleColumn} />
      </div>
      <div
        ref={containerRef}
        role="rowgroup"
        aria-label={t('fontView.tableRows')}
        tabIndex={0}
        className={styles.scroll}
        onFocus={() => setHasFocus(true)}
        onBlur={() => setHasFocus(false)}
        onPointerDown={onPointerDown}
        onKeyDown={onKeyDown}
        onContextMenu={onContextMenu}
        onDoubleClick={openFocused}
        onScroll={() => {
          onScroll()
          if (headerRef.current && containerRef.current)
            headerRef.current.scrollLeft = containerRef.current.scrollLeft
        }}
      >
        {pager.total === 0 ? (
          <p className={styles.empty}>{t('fontView.noMatches')}</p>
        ) : null}
        <div
          className={styles.content}
          style={{ height: metrics.contentHeight }}
        >
          {rows}
        </div>
      </div>
      {menu ? (
        <GlyphContextMenu
          x={menu.x}
          y={menu.y}
          commands={[
            'glyph.open',
            'glyph.updateInfo',
            'glyph.renameBatch',
            'edit.copyGlyphNames',
          ]}
          colorLabels
          onClose={() => {
            setMenu(null)
            containerRef.current?.focus()
          }}
        />
      ) : null}
    </div>
  )
}
