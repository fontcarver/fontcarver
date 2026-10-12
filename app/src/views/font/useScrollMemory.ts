/**
 * Keeps the glyph grid where the user left it.
 *
 * Two things move the grid back to the top on their own: a full reload, which
 * empties the list until the first page returns, and a tab switch, which
 * unmounts the grid altogether. Both are handled the same way — the offset is
 * recorded while the list is loaded and written back once it knows its length
 * again.
 */

import { useEffect, useRef, type RefObject } from 'react'

import type { GlyphQuery } from './glyphSelection'
import { rememberScroll, rememberedScroll } from './scrollMemory'

/** What {@link useScrollMemory} needs from the grid. */
export interface ScrollMemoryOptions {
  /** The list the offset belongs to. */
  query: GlyphQuery
  /** Scroll positions belong to one presentation of the query. */
  presentation?: 'grid' | 'list'
  containerRef: RefObject<HTMLElement | null>
  /** Offset the virtual grid currently renders at; changes once per row. */
  scrollTop: number
  /** Whether the list knows its length; `false` while a reload is in flight. */
  loaded: boolean
  /** Tells the virtual grid to re-read the offset after a restore. */
  onScroll: () => void
}

/** Records and restores the scroll offset of one glyph list. */
export function useScrollMemory({
  query,
  presentation = 'grid',
  containerRef,
  scrollTop,
  loaded,
  onScroll,
}: ScrollMemoryOptions): void {
  /** Offset still to be written back, or `null` once the grid sits on it. */
  const pending = useRef<number | null>(rememberedScroll(query, presentation))

  useEffect(() => {
    const element = containerRef.current
    if (!element) return

    if (!loaded) {
      // A reload is in flight: the content has no height, so the browser has
      // already clamped the offset to zero. Re-arm the restore and stop
      // recording, or the clamped zero would overwrite the memory.
      pending.current ??= rememberedScroll(query, presentation)
      return
    }

    const wanted = pending.current
    if (wanted === null) {
      rememberScroll(query, element.scrollTop, presentation)
      return
    }
    pending.current = null
    if (wanted === 0 || element.scrollTop === wanted) return
    element.scrollTop = wanted
    onScroll()
  }, [containerRef, loaded, onScroll, presentation, query, scrollTop])
}
