/**
 * Keyboard navigation for the glyph grid.
 *
 * Arrows move by one cell and one row, Page Up / Page Down by a screen minus
 * three rows of context, Home and End to either end of the filter; Shift
 * extends from the anchor, Enter opens the focused glyph and Escape drops the
 * selection. Every move goes through the selection hook, so a jump into a page
 * that is not cached still resolves its handles.
 */

import { useCallback, type KeyboardEvent as ReactKeyboardEvent } from 'react'

import type { DocId } from '../../api'
import { openGlyphTab } from '../../shell/tabActions'

import type { GlyphPager } from './glyphPager'
import type { GlyphSelection } from './useGlyphSelection'
import type { GridMetrics } from './useVirtualGrid'

/** Rows of the previous screen kept in view when paging. */
const PAGE_OVERLAP_ROWS = 3

/** What {@link useGridKeys} needs to move the focus. */
export interface GridKeysOptions {
  doc: DocId
  pager: GlyphPager
  metrics: GridMetrics
  /** Cells in the current filter. */
  total: number
  selection: GlyphSelection
}

/** The keyboard handler and the open action the double click shares. */
export interface GridKeys {
  /** Opens the focused cell in a glyph tab. */
  openFocused: () => void
  onKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void
}

export function useGridKeys({
  doc,
  pager,
  metrics,
  total,
  selection,
}: GridKeysOptions): GridKeys {
  const { focusIndex, focusAt, clear } = selection

  const openFocused = useCallback(() => {
    const cell = pager.cell(focusIndex)
    if (cell) openGlyphTab(doc, cell.gid, cell.name)
  }, [doc, focusIndex, pager])

  const onKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      const cols = Math.max(1, metrics.cols)
      const rows = metrics.lastRow - metrics.firstRow - PAGE_OVERLAP_ROWS
      const page = Math.max(1, rows) * cols
      const moves: Record<string, number> = {
        ArrowLeft: -1,
        ArrowRight: 1,
        ArrowUp: -cols,
        ArrowDown: cols,
        PageUp: -page,
        PageDown: page,
      }
      const delta = moves[event.key]
      if (delta !== undefined) {
        event.preventDefault()
        focusAt(focusIndex + delta, event.shiftKey)
        return
      }
      if (event.key === 'Home' || event.key === 'End') {
        event.preventDefault()
        focusAt(event.key === 'Home' ? 0 : total - 1, event.shiftKey)
        return
      }
      if (event.key === 'Enter') {
        event.preventDefault()
        openFocused()
        return
      }
      if (event.key === 'Escape') {
        event.preventDefault()
        clear()
      }
    },
    [clear, focusAt, focusIndex, metrics, openFocused, total],
  )

  return { openFocused, onKeyDown }
}
