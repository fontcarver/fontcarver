import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import { useTranslation } from 'react-i18next'

import { call, onDocumentEvent, type DocSummary, type GlyphId } from '../../api'
import { pathCache } from '../../canvas/pathCache'
import { openGlyphTab } from '../../shell/tabActions'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { useUiStore } from '../../stores/uiStore'

import { GlyphCell } from './GlyphCell'
import { GlyphContextMenu } from './GlyphContextMenu'
import styles from './fontView.module.css'
import { startFrameMeter } from './frameMeter'
import { forgetGlyphNames, rememberGlyphNames } from './glyphNames'
import { GlyphPager } from './glyphPager'
import type { GlyphQuery } from './glyphSelection'
import { useGlyphSelection } from './useGlyphSelection'
import { uiFontFamily, useCellTheme } from './useCellTheme'
import { useFontMetrics } from './useFontMetrics'
import { cellBox, indicesInRect, useVirtualGrid } from './useVirtualGrid'

/** Space between cells and around the grid, in CSS pixels. */
export const GRID_GAP = 8

/** Commands offered by the grid's context menu, in order. */
const MENU_COMMANDS = ['glyph.open', 'edit.copyGlyphNames'] as const

/** Pointer capture is best-effort: a synthetic pointer id has nothing to capture. */
function capturePointer(element: HTMLElement, pointerId: number): void {
  try {
    element.setPointerCapture(pointerId)
  } catch {
    // No active pointer with this id; the rubber band still tracks the moves.
  }
}

function releasePointer(element: HTMLElement | null, pointerId: number): void {
  try {
    element?.releasePointerCapture(pointerId)
  } catch {
    // The capture was never established.
  }
}

export interface GlyphGridProps {
  summary: DocSummary
  /** The filtered, sorted list the grid shows. Changing it remounts the grid. */
  query: GlyphQuery
}

interface Band {
  fromX: number
  fromY: number
  toX: number
  toY: number
}

interface MenuState {
  x: number
  y: number
}

/**
 * The virtualised glyph grid.
 *
 * Only the cells inside the viewport (plus two rows of overscan) exist in the
 * DOM, and their contents come from `glyphs.list` pages requested for that
 * range alone. Pointer and keyboard input is handled here and mapped to cells
 * by position, so no cell carries an event handler of its own. The component is
 * mounted under a key derived from the query, so a new filter, sort or master
 * starts from a clean page cache at the top of the list.
 */
export function GlyphGrid({ summary, query }: GlyphGridProps) {
  const { t } = useTranslation()
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const setMatchCount = useDocumentStore((state) => state.setMatchCount)

  const containerRef = useRef<HTMLDivElement>(null)
  const [version, bump] = useReducer((count: number) => count + 1, 0)
  const [band, setBand] = useState<Band | null>(null)
  const [menu, setMenu] = useState<MenuState | null>(null)
  const [hasFocus, setHasFocus] = useState(false)

  const [pager] = useState(
    () =>
      new GlyphPager({
        fetchPage: async (offset, limit) => {
          const result = await call('glyphs.list', {
            doc: query.doc,
            master: query.master,
            filter: query.filter,
            sort: query.sort,
            offset,
            limit,
            include_paths: true,
          })
          rememberGlyphNames(query.doc, result.items)
          return result
        },
        onChange: bump,
        onError: (error) =>
          useUiStore.getState().pushToast({
            kind: 'error',
            message: error instanceof Error ? error.message : String(error),
          }),
      }),
  )

  const theme = useCellTheme()
  const em = useFontMetrics(doc, query.master, summary.units_per_em)
  const fontFamily = useMemo(() => uiFontFamily(), [])
  const dpr = globalThis.devicePixelRatio || 1
  const total = pager.total ?? 0

  const { metrics, scrollTo, onScroll } = useVirtualGrid({
    total,
    cellSize: view.cellSize,
    gap: GRID_GAP,
    containerRef,
  })

  const { focusIndex, apply, selectAt, focusAt, anchorAt, clear } =
    useGlyphSelection({ doc, query, pager, total, scrollTo })

  useEffect(() => startFrameMeter(), [])

  useEffect(() => () => pager.cancel(), [pager])

  useEffect(() => {
    if (metrics.last < metrics.first) pager.setRange(0, 0)
    else pager.setRange(metrics.first, metrics.last)
  }, [metrics.first, metrics.last, pager, version])

  useEffect(() => {
    if (pager.total !== null) setMatchCount(doc, pager.total)
  }, [doc, pager, setMatchCount, version])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc) return
        if (event.change.glyph_order) {
          forgetGlyphNames(doc)
          pathCache.clearDoc(doc)
          pager.reset()
          pager.setRange(0, 0)
          return
        }
        if (event.change.glyphs.length === 0) return
        pathCache.drop(doc, event.change.glyphs)
        pager.invalidate(event.change.glyphs)
      }),
    [doc, pager],
  )

  const contentPoint = useCallback(
    (clientX: number, clientY: number): { x: number; y: number } | null => {
      const element = containerRef.current
      if (!element) return null
      const bounds = element.getBoundingClientRect()
      return {
        x: clientX - bounds.left + element.scrollLeft,
        y: clientY - bounds.top + element.scrollTop,
      }
    },
    [],
  )

  /**
   * The cell under a pointer event. Read from the DOM rather than recomputed
   * from geometry: the cells carry their index, so a hit test is a `closest`
   * call and needs no layout measurement.
   */
  const indexAt = useCallback((target: EventTarget | null): number | null => {
    if (!(target instanceof Element)) return null
    const raw = target.closest('[data-index]')?.getAttribute('data-index')
    if (raw === null || raw === undefined) return null
    const index = Number(raw)
    return Number.isInteger(index) ? index : null
  }, [])

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) return
      const element = containerRef.current
      if (!element) return
      setMenu(null)
      element.focus()
      const index = indexAt(event.target)
      const additive = event.ctrlKey || event.metaKey

      if (index === null) {
        const point = contentPoint(event.clientX, event.clientY)
        if (!point) return
        if (!additive && !event.shiftKey) clear()
        setBand({ fromX: point.x, fromY: point.y, toX: point.x, toY: point.y })
        capturePointer(element, event.pointerId)
        return
      }

      selectAt(index, { shift: event.shiftKey, additive })
    },
    [clear, contentPoint, indexAt, selectAt],
  )

  const onPointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (!band) return
      const point = contentPoint(event.clientX, event.clientY)
      if (!point) return
      setBand({ ...band, toX: point.x, toY: point.y })
    },
    [band, contentPoint],
  )

  const onPointerUp = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (!band) return
      releasePointer(containerRef.current, event.pointerId)
      const indices = indicesInRect(
        {
          x: band.fromX,
          y: band.fromY,
          width: band.toX - band.fromX,
          height: band.toY - band.fromY,
        },
        metrics,
        view.cellSize,
        GRID_GAP,
        total,
      )
      setBand(null)
      const first = indices[0]
      if (first === undefined) return
      const gids = indices
        .map((index) => pager.cell(index)?.gid)
        .filter((gid): gid is GlyphId => gid !== undefined)
      apply(gids, event.ctrlKey || event.metaKey || event.shiftKey)
      anchorAt(first)
    },
    [anchorAt, apply, band, metrics, pager, total, view.cellSize],
  )

  const closeMenu = useCallback(() => setMenu(null), [])

  const openFocused = useCallback(() => {
    const gid = pager.cell(focusIndex)?.gid
    if (gid !== undefined) openGlyphTab(doc, gid)
  }, [doc, focusIndex, pager])

  const onKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      const cols = Math.max(1, metrics.cols)
      const page = Math.max(1, metrics.lastRow - metrics.firstRow - 3) * cols
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

  const onContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      const index = indexAt(event.target)
      if (index === null) return
      event.preventDefault()
      const gid = pager.cell(index)?.gid
      const selected =
        gid !== undefined &&
        (useDocumentStore.getState().byDoc[doc]?.selection.has(gid) ?? false)
      if (selected) anchorAt(index)
      else selectAt(index, { shift: false, additive: false })
      setMenu({ x: event.clientX, y: event.clientY })
    },
    [anchorAt, doc, indexAt, pager, selectAt],
  )

  const cells = []
  for (let index = metrics.first; index <= metrics.last; index += 1) {
    const cell = pager.cell(index)
    const box = cellBox(index, metrics, view.cellSize, GRID_GAP)
    cells.push(
      <GlyphCell
        key={index}
        doc={doc}
        cell={cell}
        index={index}
        total={total}
        x={box.x}
        y={box.y}
        size={view.cellSize}
        dpr={dpr}
        em={em}
        labelMode={view.labelMode}
        selected={cell !== undefined && view.selection.has(cell.gid)}
        focused={hasFocus && index === focusIndex}
        theme={theme}
        fontFamily={fontFamily}
      />,
    )
  }

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
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onDoubleClick={openFocused}
      onContextMenu={onContextMenu}
      onKeyDown={onKeyDown}
    >
      {pager.total === 0 ? (
        <p className={styles.empty}>{t('fontView.noMatches')}</p>
      ) : null}
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
      {menu ? (
        <GlyphContextMenu
          x={menu.x}
          y={menu.y}
          commands={MENU_COMMANDS}
          onClose={closeMenu}
        />
      ) : null}
    </div>
  )
}
