import { useEffect, useMemo, useRef, useState } from 'react'

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
  const cache = useRef<{ identity: string; cells: Map<GlyphId, GlyphCell> }>({
    identity,
    cells: new Map(),
  })
  const [version, setVersion] = useState(0)

  if (cache.current.identity !== identity)
    cache.current = { identity, cells: new Map() }

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
    const missing = wanted.filter((gid) => !cache.current.cells.has(gid))
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
          if (cancelled || cache.current.identity !== identity) return
          cache.current.cells.set(gid, cell)
          setVersion((value) => value + 1)
        } catch (error) {
          if (!cancelled) reportError(error)
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
  }, [doc, identity, master, signature, wanted])

  return useMemo(() => new Map(cache.current.cells), [identity, version])
}
