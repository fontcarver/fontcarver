import { useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocSummary, GlyphId } from '../../api'
import { changeGlyphTab } from '../../shell/tabActions'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { GRID_GAP } from '../font/GlyphGrid'
import { GridCells } from '../font/GridCells'
import type { GlyphQuery } from '../font/glyphSelection'
import { indexOf } from '../font/glyphOrder'
import { uiFontFamily, useCellTheme } from '../font/useCellTheme'
import { useFontMetrics } from '../font/useFontMetrics'
import { useGridPager } from '../font/useGridPager'
import { useVirtualGrid } from '../font/useVirtualGrid'

import styles from './editSidebar.module.css'

/** Cell edge of the compact list, in CSS pixels (design spec §3). */
export const COMPACT_CELL = 36

export interface EditGlyphListProps {
  summary: DocSummary
  /** The glyph the Edit tab is showing, which the list marks and scrolls to. */
  gid: GlyphId
  /**
   * The filtered, sorted list to show. Like the Font view's grid, this
   * component is mounted under a key derived from the query, so the page cache
   * inside it always belongs to the list on screen.
   */
  query: GlyphQuery
}

/**
 * The Edit tab's compact glyph list (design spec §3).
 *
 * It is the Font view's own grid at a small cell size and under the Font
 * view's current filter, so "next glyph" in the editor and "next glyph" in the
 * grid mean the same thing. Clicking a cell replaces the glyph of *this* tab
 * rather than opening another one, which is what keeps a long editing session
 * to a single Edit tab.
 */
export function EditGlyphList({ summary, gid, query }: EditGlyphListProps) {
  const { t } = useTranslation()
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const containerRef = useRef<HTMLDivElement>(null)

  const { pager, version, total } = useGridPager({ doc, query })
  const theme = useCellTheme()
  const em = useFontMetrics(doc, query.master, summary.units_per_em)
  const fontFamily = useMemo(() => uiFontFamily(), [])
  const dpr = globalThis.devicePixelRatio || 1

  const { metrics, scrollTo, onScroll } = useVirtualGrid({
    total,
    cellSize: COMPACT_CELL,
    gap: GRID_GAP,
    containerRef,
  })

  useEffect(() => {
    if (metrics.last < metrics.first) pager.setRange(0, 0)
    else pager.setRange(metrics.first, metrics.last)
  }, [metrics.first, metrics.last, pager, version])

  // Follow the tab: opening a glyph from anywhere brings its cell into view.
  useEffect(() => {
    let cancelled = false
    void indexOf(query, gid)
      .then((index) => {
        if (!cancelled && index !== undefined) scrollTo(index)
      })
      .catch(() => {
        // Scrolling the list to the open glyph is a convenience: a position
        // the kernel cannot resolve leaves the list where the user left it.
      })
    return () => {
      cancelled = true
    }
  }, [gid, query, scrollTo])

  const selection = useMemo(() => new Set([gid]), [gid])

  return (
    <div
      ref={containerRef}
      className={styles.glyphScroll}
      role="listbox"
      aria-label={t('panel.sidebar.glyphs')}
      tabIndex={0}
      onScroll={onScroll}
      onClick={(event) => {
        const cell = (event.target as HTMLElement).closest('[data-index]')
        const index = Number(cell?.getAttribute('data-index') ?? -1)
        const target = pager.cell(index)
        if (!target || target.gid === gid) return
        void changeGlyphTab(doc, gid, target.gid, target.name)
      }}
    >
      <GridCells
        doc={doc}
        pager={pager}
        metrics={metrics}
        cellSize={COMPACT_CELL}
        gap={GRID_GAP}
        total={total}
        dpr={dpr}
        em={em}
        labelMode={view.labelMode}
        selection={selection}
        focusIndex={-1}
        hasFocus={false}
        theme={theme}
        fontFamily={fontFamily}
        band={null}
      />
    </div>
  )
}
