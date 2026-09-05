import { useLayoutEffect, useRef, useState } from 'react'

import type { GlyphCell } from '../api'

import styles from './dialogs.module.css'

/** Row height of the preview list, in CSS pixels. */
const ROW_HEIGHT = 20

/** Rows kept above and below the viewport so scrolling never shows a gap. */
const OVERSCAN = 4

/** Viewport height assumed before the element has been measured. */
const FALLBACK_HEIGHT = 160

export interface NameListProps {
  /** Cells to list, in the order the kernel would create them. */
  cells: readonly GlyphCell[]
  /** Accessible name of the list. */
  label: string
  /** Shown instead of the list when there is nothing to preview. */
  empty: string
}

/**
 * Virtualised list of glyph names. A GB 2312 recipe previews about 6,800 rows,
 * so only the visible window is mounted; the rows are absolutely placed inside
 * a spacer of the full height so the scrollbar stays honest.
 */
export function NameList({ cells, label, empty }: NameListProps) {
  const viewportRef = useRef<HTMLDivElement>(null)
  const [scrollTop, setScrollTop] = useState(0)
  const [height, setHeight] = useState(FALLBACK_HEIGHT)

  useLayoutEffect(() => {
    const measured = viewportRef.current?.clientHeight
    if (measured) setHeight(measured)
  }, [])

  if (cells.length === 0) return <p className={styles.previewEmpty}>{empty}</p>

  const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN)
  const last = Math.min(
    cells.length,
    first + Math.ceil(height / ROW_HEIGHT) + OVERSCAN * 2,
  )

  return (
    <div
      ref={viewportRef}
      role="list"
      aria-label={label}
      className={styles.preview}
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
    >
      <div
        className={styles.previewSpacer}
        style={{ height: `${cells.length * ROW_HEIGHT}px` }}
      >
        {cells.slice(first, last).map((cell, index) => (
          <div
            key={cell.name}
            role="listitem"
            className={styles.previewRow}
            style={{ top: `${(first + index) * ROW_HEIGHT}px` }}
          >
            <span>{cell.name}</span>
            {cell.unicodes.length > 0 ? (
              <span className={styles.previewCode}>
                {cell.unicodes
                  .map(
                    (code) =>
                      `U+${code.toString(16).toUpperCase().padStart(4, '0')}`,
                  )
                  .join(' ')}
              </span>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  )
}
