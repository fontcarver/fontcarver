import { useEffect, useMemo, useState } from 'react'

import {
  call,
  type DocId,
  type GlyphCell,
  type GlyphId,
  type MasterId,
} from '../../api'
import { reportError } from '../../shell/commands'

import type { PositionedGlyph } from './textLayout'

const CELL_CONCURRENCY = 8

/** Loads only the distinct cells requested by the canvas and preview windows. */
export function useTextGlyphCells(
  doc: DocId,
  master: MasterId | null,
  generation: number,
  glyphs: readonly PositionedGlyph[],
  indexes: readonly number[],
): ReadonlyMap<GlyphId, GlyphCell> {
  const identity = `${doc}:${master ?? ''}:${generation}`
  const cache = useMemo(
    () => ({ identity, cells: new Map<GlyphId, GlyphCell>() }),
    [identity],
  )
  const [loaded, setLoaded] = useState({
    identity: '',
    cells: new Map<GlyphId, GlyphCell>(),
  })
  const wanted = useMemo(
    () =>
      [
        ...new Set(
          indexes.flatMap((index) => {
            const gid = glyphs[index]?.documentGid
            return gid === null || gid === undefined ? [] : [gid]
          }),
        ),
      ].sort((left, right) => left - right),
    [glyphs, indexes],
  )
  const signature = wanted.join(',')

  useEffect(() => {
    if (master === null) return
    const requested = signature.split(',').filter(Boolean).map(Number)
    const missing = requested.filter((gid) => !cache.cells.has(gid))
    if (missing.length === 0) return
    let cancelled = false
    let next = 0
    const worker = async (): Promise<void> => {
      while (!cancelled && next < missing.length) {
        const gid = missing[next]
        next += 1
        if (gid === undefined) continue
        try {
          const cell = await call('glyphs.cell', { doc, gid, master })
          if (cancelled || cache.identity !== identity) return
          cache.cells.set(gid, cell)
          setLoaded({ identity, cells: new Map(cache.cells) })
        } catch (error) {
          if (!cancelled) {
            cancelled = true
            reportError(error)
          }
        }
      }
    }
    const workers = Array.from(
      { length: Math.min(CELL_CONCURRENCY, missing.length) },
      () => worker(),
    )
    void Promise.all(workers)
    return () => {
      cancelled = true
    }
  }, [cache, doc, identity, master, signature])

  return loaded.identity === identity ? loaded.cells : EMPTY_CELLS
}

const EMPTY_CELLS: ReadonlyMap<GlyphId, GlyphCell> = new Map()
