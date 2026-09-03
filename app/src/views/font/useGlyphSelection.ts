/**
 * Selection and focus for the glyph grid.
 *
 * The grid navigates by position in the filtered, sorted order while the store
 * holds `GlyphId`s, so every operation here translates positions into handles,
 * falling back to the kernel when the cells are not in the page cache.
 */

import { useCallback, useEffect, useRef, useState } from 'react'

import type { DocId, GlyphId } from '../../api'
import { useDocumentStore } from '../../stores/documentStore'

import type { GlyphPager } from './glyphPager'
import {
  fetchGidRange,
  rangeBetween,
  withAdded,
  withToggled,
  type GlyphQuery,
} from './glyphSelection'

export interface GlyphSelectionOptions {
  doc: DocId
  query: GlyphQuery
  pager: GlyphPager
  /** Number of cells in the current filter. */
  total: number
  /** Brings one cell into view; supplied by the virtual grid. */
  scrollTo: (index: number) => void
}

export interface GlyphSelection {
  /** Cell the keyboard acts on. */
  focusIndex: number
  /** Replaces the selection with the given handles, or adds to it. */
  apply: (gids: readonly GlyphId[], additive: boolean) => void
  /** Selects one cell: plain, Shift-extended from the anchor, or toggled. */
  selectAt: (
    index: number,
    modifiers: { shift: boolean; additive: boolean },
  ) => void
  /** Moves the focus, scrolls it into view and selects it or extends to it. */
  focusAt: (index: number, extend: boolean) => void
  /** Sets the focus and the anchor without touching the selection. */
  anchorAt: (index: number) => void
  /** Empties the selection. */
  clear: () => void
}

export function useGlyphSelection({
  doc,
  query,
  pager,
  total,
  scrollTo,
}: GlyphSelectionOptions): GlyphSelection {
  const setSelection = useDocumentStore((state) => state.setSelection)
  const [focusIndex, setFocusIndex] = useState(0)
  const anchorRef = useRef(0)
  /**
   * Counter identifying the newest selection request. A range that has to ask
   * the kernel for its handles can resolve after a later click or key press,
   * so every request captures this value and only the current one may write.
   */
  const requestRef = useRef(0)

  useEffect(
    () => () => {
      // Unmounting (a new filter, sort or master) retires the pending request.
      requestRef.current += 1
    },
    [],
  )

  const apply = useCallback(
    (gids: readonly GlyphId[], additive: boolean) => {
      requestRef.current += 1
      const current = useDocumentStore.getState().byDoc[doc]?.selection
      setSelection(doc, additive && current ? withAdded(current, gids) : gids)
    },
    [doc, setSelection],
  )

  const gidsFor = useCallback(
    async (first: number, last: number): Promise<GlyphId[]> => {
      const cached: GlyphId[] = []
      let complete = true
      for (let index = first; index <= last && complete; index += 1) {
        const cell = pager.cell(index)
        if (cell) cached.push(cell.gid)
        else complete = false
      }
      if (complete) return cached
      return fetchGidRange(query, first, last - first + 1)
    },
    [pager, query],
  )

  const selectRange = useCallback(
    async (from: number, to: number, additive: boolean) => {
      const [first, last] = rangeBetween(from, to)
      const request = (requestRef.current += 1)
      const gids = await gidsFor(first, last)
      if (request !== requestRef.current) return
      apply(gids, additive)
    },
    [apply, gidsFor],
  )

  const selectAt = useCallback(
    (
      index: number,
      { shift, additive }: { shift: boolean; additive: boolean },
    ) => {
      setFocusIndex(index)
      if (shift) {
        void selectRange(anchorRef.current, index, additive)
        return
      }
      anchorRef.current = index
      const gid = pager.cell(index)?.gid
      if (gid === undefined) return
      requestRef.current += 1
      if (!additive) {
        setSelection(doc, [gid])
        return
      }
      const current = useDocumentStore.getState().byDoc[doc]?.selection
      setSelection(doc, current ? withToggled(current, gid) : [gid])
    },
    [doc, pager, selectRange, setSelection],
  )

  const focusAt = useCallback(
    (index: number, extend: boolean) => {
      const clamped = Math.min(Math.max(0, index), Math.max(0, total - 1))
      setFocusIndex(clamped)
      scrollTo(clamped)
      if (!extend) anchorRef.current = clamped
      void selectRange(anchorRef.current, clamped, false)
    },
    [scrollTo, selectRange, total],
  )

  const anchorAt = useCallback((index: number) => {
    anchorRef.current = index
    setFocusIndex(index)
  }, [])

  const clear = useCallback(() => {
    requestRef.current += 1
    setSelection(doc, [])
  }, [doc, setSelection])

  return { focusIndex, apply, selectAt, focusAt, anchorAt, clear }
}
