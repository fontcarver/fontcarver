import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocSummary } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'

import { GlyphContextMenu } from './GlyphContextMenu'
import { GridCells } from './GridCells'
import styles from './fontView.module.css'
import { startFrameMeter } from './frameMeter'
import type { GlyphQuery } from './glyphSelection'
import { uiFontFamily, useCellTheme } from './useCellTheme'
import { useFontMetrics } from './useFontMetrics'
import { useGlyphSelection } from './useGlyphSelection'
import { useGridKeys } from './useGridKeys'
import { useGridPager } from './useGridPager'
import { useGridPointer } from './useGridPointer'
import { useScrollMemory } from './useScrollMemory'
import { useVirtualGrid } from './useVirtualGrid'

/** Space between cells and around the grid, in CSS pixels. */
export const GRID_GAP = 8

/** Commands offered by the grid's context menu, in order. */
const MENU_COMMANDS = [
  'glyph.open',
  'glyph.renameBatch',
  'edit.copyGlyphNames',
] as const

export interface GlyphGridProps {
  summary: DocSummary
  /** The filtered, sorted list the grid shows. Changing it remounts the grid. */
  query: GlyphQuery
}

/**
 * The virtualised glyph grid.
 *
 * Only the cells inside the viewport (plus two rows of overscan) exist in the
 * DOM, and their contents come from `glyphs.list` pages requested for that
 * range alone. The component itself only wires the pieces together: paging and
 * invalidation in `useGridPager`, geometry in `useVirtualGrid`, input in
 * `useGridPointer` and `useGridKeys`, painting in `GridCells`. It is mounted
 * under a key derived from the query, so a new filter, sort or master starts
 * from a clean page cache.
 */
export function GlyphGrid({ summary, query }: GlyphGridProps) {
  const { t } = useTranslation()
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const setFocus = useDocumentStore((state) => state.setFocus)

  const containerRef = useRef<HTMLDivElement>(null)
  const [hasFocus, setHasFocus] = useState(false)

  const { pager, version, total } = useGridPager({ doc, query })
  const theme = useCellTheme()
  const em = useFontMetrics(doc, query.master, summary.units_per_em)
  const fontFamily = useMemo(() => uiFontFamily(), [])
  const dpr = globalThis.devicePixelRatio || 1

  const { metrics, scrollTop, scrollTo, onScroll } = useVirtualGrid({
    total,
    cellSize: view.cellSize,
    gap: GRID_GAP,
    containerRef,
  })

  useScrollMemory({
    query,
    containerRef,
    scrollTop,
    loaded: pager.total !== null,
    onScroll,
  })

  const selection = useGlyphSelection({ doc, query, pager, total, scrollTo })
  const pointer = useGridPointer({
    doc,
    containerRef,
    pager,
    metrics,
    cellSize: view.cellSize,
    gap: GRID_GAP,
    total,
    selection,
  })
  const { openFocused, onKeyDown } = useGridKeys({
    doc,
    pager,
    metrics,
    total,
    selection,
  })

  useEffect(() => startFrameMeter(), [])

  useEffect(() => {
    if (metrics.last < metrics.first) pager.setRange(0, 0)
    else pager.setRange(metrics.first, metrics.last)
  }, [metrics.first, metrics.last, pager, version])

  // Keeps the store's focus glyph in step with the focus cell, so glyph-scoped
  // commands (the menu bar, the palette, the context menu) act on the cell the
  // user last touched rather than on the first member of the selection.
  useEffect(() => {
    setFocus(doc, pager.cell(selection.focusIndex)?.gid ?? null)
  }, [doc, pager, selection.focusIndex, setFocus, version])

  return (
    <div
      ref={containerRef}
      className={styles.scroll}
      role="listbox"
      aria-multiselectable="true"
      aria-label={t('fontView.gridLabel')}
      tabIndex={0}
      onScroll={onScroll}
      onFocus={() => setHasFocus(true)}
      onBlur={() => setHasFocus(false)}
      onPointerDown={pointer.onPointerDown}
      onPointerMove={pointer.onPointerMove}
      onPointerUp={pointer.onPointerUp}
      onDoubleClick={openFocused}
      onContextMenu={pointer.onContextMenu}
      onKeyDown={onKeyDown}
    >
      {pager.total === 0 ? (
        <p className={styles.empty}>{t('fontView.noMatches')}</p>
      ) : null}
      <GridCells
        doc={doc}
        pager={pager}
        metrics={metrics}
        cellSize={view.cellSize}
        gap={GRID_GAP}
        total={total}
        dpr={dpr}
        em={em}
        labelMode={view.labelMode}
        selection={view.selection}
        focusIndex={selection.focusIndex}
        hasFocus={hasFocus}
        theme={theme}
        fontFamily={fontFamily}
        band={pointer.band}
      />
      {pointer.menu ? (
        <GlyphContextMenu
          x={pointer.menu.x}
          y={pointer.menu.y}
          commands={MENU_COMMANDS}
          onClose={pointer.closeMenu}
        />
      ) : null}
    </div>
  )
}
