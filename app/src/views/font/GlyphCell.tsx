import { memo, useCallback, useEffect, useRef, useState } from 'react'

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
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  /**
   * Bumped whenever a canvas attaches.
   *
   * The paint below has to run *after* the element exists, and a ref alone
   * gives no such guarantee: a cell whose page was invalidated renders its
   * placeholder and then a *new* canvas, and the old node's ref cleanup can
   * arrive after the new one attached — which used to leave the paint with no
   * canvas and the cell permanently blank. Counting attachments makes the
   * effect depend on the node without making the node itself state, which
   * would put a mutable DOM object under the compiler's immutability rule.
   */
  const [attached, setAttached] = useState(0)

  /**
   * A 65k-glyph scroll churns through thousands of cells, so the backing store
   * is released the moment the canvas detaches rather than left to the
   * collector.
   */
  const attachCanvas = useCallback((node: HTMLCanvasElement | null) => {
    canvasRef.current = node
    setAttached((count) => count + 1)
    return () => {
      // Only clear the handle this cleanup is for.
      if (canvasRef.current === node) canvasRef.current = null
      if (!node) return
      node.width = 0
      node.height = 0
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
    attached,
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
        ref={attachCanvas}
        className={styles.cellCanvas}
        style={{ width: size, height: size }}
      />
    </div>
  )
}

/** Memoised so scrolling only repaints the cells that actually changed. */
export const GlyphCell = memo(GlyphCellView)
