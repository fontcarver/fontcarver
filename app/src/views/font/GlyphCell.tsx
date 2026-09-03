import { memo, useEffect, useRef } from 'react'

import type { DocId, GlyphCell as GlyphCellData } from '../../api'
import type { EmBox } from '../../canvas/glyphPaint'
import { pathCache } from '../../canvas/pathCache'
import type { LabelMode } from '../../stores/documentStore'

import { cellLabel, paintCell, type CellTheme } from './cellRenderer'
import styles from './fontView.module.css'

export interface GlyphCellProps {
  /** Document the cell belongs to; glyph handles are unique per document. */
  doc: DocId
  /** The loaded cell, or `undefined` while its page is still in flight. */
  cell: GlyphCellData | undefined
  /** Position in the current filtered, sorted order. */
  index: number
  /** Number of cells in the current filter, for assistive technology. */
  total: number
  x: number
  y: number
  size: number
  dpr: number
  em: EmBox
  labelMode: LabelMode
  selected: boolean
  focused: boolean
  theme: CellTheme
  fontFamily: string
}

/**
 * One glyph-grid cell: a device-pixel-ratio aware canvas painted by
 * `paintCell`. The grid handles pointer and keyboard input, so the cell itself
 * carries only its identity and position.
 */
function GlyphCellView({
  doc,
  cell,
  index,
  total,
  x,
  y,
  size,
  dpr,
  em,
  labelMode,
  selected,
  focused,
  theme,
  fontFamily,
}: GlyphCellProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    return () => {
      // A 65k-glyph scroll churns through thousands of cells; releasing the
      // backing store on unmount frees the pixels without waiting for GC.
      if (!canvas) return
      canvas.width = 0
      canvas.height = 0
    }
  }, [])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || !cell) return
    const device = Math.max(1, Math.round(size * dpr))
    if (canvas.width !== device) canvas.width = device
    if (canvas.height !== device) canvas.height = device
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    paintCell(ctx, cell, {
      size,
      dpr,
      upem: em.upem,
      ascender: em.ascender,
      descender: em.descender,
      labelMode,
      index,
      selected,
      focused,
      theme,
      path: pathCache.get(doc, cell.gid, cell.generation, cell.path),
      fontFamily,
    })
  }, [
    cell,
    doc,
    dpr,
    em.ascender,
    em.descender,
    em.upem,
    focused,
    fontFamily,
    index,
    labelMode,
    selected,
    size,
    theme,
  ])

  const style = {
    transform: `translate(${x}px, ${y}px)`,
    width: size,
    height: size,
  }

  if (!cell) {
    return (
      <div
        className={styles.pending}
        style={style}
        data-index={index}
        aria-hidden="true"
      />
    )
  }

  return (
    <div
      role="option"
      aria-selected={selected}
      aria-label={cellLabel(cell, 'name', index)}
      aria-posinset={index + 1}
      aria-setsize={total}
      title={cell.name}
      data-index={index}
      data-gid={cell.gid}
      data-glyph={cell.name}
      className={styles.cell}
      style={style}
    >
      <canvas
        ref={canvasRef}
        className={styles.cellCanvas}
        style={{ width: size, height: size }}
      />
    </div>
  )
}

/** Memoised so scrolling only repaints the cells that actually changed. */
export const GlyphCell = memo(GlyphCellView)
