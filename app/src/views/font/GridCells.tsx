import type { DocId, GlyphId } from '../../api'
import type { EmBox } from '../../canvas/glyphPaint'
import type { LabelMode } from '../../stores/documentStore'

import type { CellTheme } from './cellRenderer'
import { GlyphCell } from './GlyphCell'
import styles from './fontView.module.css'
import type { GlyphPager } from './glyphPager'
import type { Band } from './useGridPointer'
import { cellBox, type GridMetrics } from './useVirtualGrid'

export interface GridCellsProps {
  doc: DocId
  pager: GlyphPager
  metrics: GridMetrics
  /** Cell edge length in CSS pixels. */
  cellSize: number
  /** Space between cells and around the grid. */
  gap: number
  /** Cells in the current filter, for assistive technology. */
  total: number
  dpr: number
  em: EmBox
  labelMode: LabelMode
  selection: ReadonlySet<GlyphId>
  focusIndex: number
  /** Whether the grid itself holds the keyboard focus. */
  hasFocus: boolean
  theme: CellTheme
  fontFamily: string
  /** The rubber band being dragged, or `null`. */
  band: Band | null
}

/**
 * The scrollable content of the grid: one absolutely positioned cell per
 * rendered index, plus the rubber band.
 *
 * The content box is as tall as the whole filter, so the scrollbar reflects the
 * font rather than the handful of cells that exist in the DOM.
 */
export function GridCells({
  doc,
  pager,
  metrics,
  cellSize,
  gap,
  total,
  dpr,
  em,
  labelMode,
  selection,
  focusIndex,
  hasFocus,
  theme,
  fontFamily,
  band,
}: GridCellsProps) {
  const cells = []
  for (let index = metrics.first; index <= metrics.last; index += 1) {
    const cell = pager.cell(index)
    const box = cellBox(index, metrics, cellSize, gap)
    cells.push(
      <GlyphCell
        key={index}
        doc={doc}
        cell={cell}
        index={index}
        total={total}
        x={box.x}
        y={box.y}
        size={cellSize}
        dpr={dpr}
        em={em}
        labelMode={labelMode}
        selected={cell !== undefined && selection.has(cell.gid)}
        focused={hasFocus && index === focusIndex}
        theme={theme}
        fontFamily={fontFamily}
      />,
    )
  }

  return (
    <div className={styles.content} style={{ height: metrics.contentHeight }}>
      {cells}
      {band ? (
        <div
          className={styles.band}
          style={{
            left: Math.min(band.fromX, band.toX),
            top: Math.min(band.fromY, band.toY),
            width: Math.abs(band.toX - band.fromX),
            height: Math.abs(band.toY - band.fromY),
          }}
        />
      ) : null}
    </div>
  )
}
