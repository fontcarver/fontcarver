/**
 * Pointer input for the glyph grid: click, Shift and Ctrl selection, the rubber
 * band and the context menu.
 *
 * The handlers live on the scroll container, never on the cells, so a 65k-glyph
 * font does not carry 65k listeners. A cell is identified from the DOM — every
 * cell renders its position in `data-index` — which makes a hit test a `closest`
 * call instead of a layout measurement.
 */

import {
  useCallback,
  useState,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
} from 'react'

import type { DocId, GlyphId } from '../../api'
import { useDocumentStore } from '../../stores/documentStore'

import type { GlyphPager } from './glyphPager'
import type { GlyphSelection } from './useGlyphSelection'
import { indicesInRect, type GridMetrics } from './useVirtualGrid'

/** The rubber band while it is being dragged, in content coordinates. */
export interface Band {
  fromX: number
  fromY: number
  toX: number
  toY: number
}

/** Where the context menu was opened, in client coordinates. */
export interface MenuPosition {
  x: number
  y: number
}

/** What {@link useGridPointer} needs to map a pointer onto cells. */
export interface GridPointerOptions {
  doc: DocId
  containerRef: RefObject<HTMLDivElement | null>
  pager: GlyphPager
  metrics: GridMetrics
  cellSize: number
  gap: number
  /** Cells in the current filter. */
  total: number
  selection: GlyphSelection
}

/** Pointer state and the handlers the scroll container attaches. */
export interface GridPointer {
  band: Band | null
  menu: MenuPosition | null
  closeMenu: () => void
  onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void
  onPointerMove: (event: ReactPointerEvent<HTMLDivElement>) => void
  onPointerUp: (event: ReactPointerEvent<HTMLDivElement>) => void
  onContextMenu: (event: ReactMouseEvent<HTMLDivElement>) => void
}

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

/** The cell position under an event target, or `null` outside every cell. */
export function indexAt(target: EventTarget | null): number | null {
  if (!(target instanceof Element)) return null
  const raw = target.closest('[data-index]')?.getAttribute('data-index')
  if (raw === null || raw === undefined) return null
  const index = Number(raw)
  return Number.isInteger(index) ? index : null
}

export function useGridPointer({
  doc,
  containerRef,
  pager,
  metrics,
  cellSize,
  gap,
  total,
  selection,
}: GridPointerOptions): GridPointer {
  const { apply, selectAt, anchorAt, clear } = selection
  const [band, setBand] = useState<Band | null>(null)
  const [menu, setMenu] = useState<MenuPosition | null>(null)

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
    [containerRef],
  )

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
    [clear, containerRef, contentPoint, selectAt],
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
        cellSize,
        gap,
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
    [anchorAt, apply, band, cellSize, containerRef, gap, metrics, pager, total],
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
      // A right click inside the selection keeps it and only moves the target,
      // so "Copy Glyph Names" still acts on everything the user picked.
      if (selected) anchorAt(index)
      else selectAt(index, { shift: false, additive: false })
      setMenu({ x: event.clientX, y: event.clientY })
    },
    [anchorAt, doc, pager, selectAt],
  )

  const closeMenu = useCallback(() => {
    setMenu(null)
    containerRef.current?.focus()
  }, [containerRef])

  return {
    band,
    menu,
    closeMenu,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onContextMenu,
  }
}
