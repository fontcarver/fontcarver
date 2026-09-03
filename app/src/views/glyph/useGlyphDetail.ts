/**
 * Loads everything one glyph tab draws.
 *
 * `glyphs.detail` carries the persistent glyph plus one flattened path per
 * layer; `glyphs.cell` adds the exact outline bounds of the selected master,
 * which is what the sidebearing readouts need. Both are re-requested when a
 * document event touches this glyph — there is no polling.
 */

import { useEffect, useState } from 'react'

import {
  call,
  onDocumentEvent,
  type DocId,
  type GlyphCell,
  type GlyphDetail,
  type GlyphId,
  type MasterId,
} from '../../api'

/** What `useGlyphDetail` reports. */
export interface GlyphDetailResult {
  detail: GlyphDetail | null
  cell: GlyphCell | null
  error: string | null
  loading: boolean
  /** Bumped whenever font-wide or master data changed, for the info cache. */
  fontGeneration: number
}

interface Loaded {
  key: string
  detail: GlyphDetail | null
  cell: GlyphCell | null
  error: string | null
}

/** Fetches and keeps fresh the detail of one glyph. */
export function useGlyphDetail(
  doc: DocId,
  gid: GlyphId | undefined,
  master: MasterId | null,
): GlyphDetailResult {
  const [reload, setReload] = useState(0)
  const [fontGeneration, setFontGeneration] = useState(0)
  const [loaded, setLoaded] = useState<Loaded>({
    key: '',
    detail: null,
    cell: null,
    error: null,
  })

  const masterKey = master ?? ''
  const key = `${doc}:${gid ?? 'none'}:${masterKey}:${reload}`

  useEffect(() => {
    if (gid === undefined) return
    let cancelled = false

    const detail = call('glyphs.detail', { doc, gid })
    // A font with no masters cannot produce a cell; the outline still draws.
    const cell =
      masterKey === ''
        ? Promise.resolve(null)
        : call('glyphs.cell', { doc, gid, master: masterKey }).catch(() => null)

    void Promise.all([detail, cell])
      .then(([loadedDetail, loadedCell]) => {
        if (cancelled) return
        setLoaded({
          key,
          detail: loadedDetail,
          cell: loadedCell,
          error: null,
        })
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setLoaded({
          key,
          detail: null,
          cell: null,
          error: error instanceof Error ? error.message : String(error),
        })
      })

    return () => {
      cancelled = true
    }
  }, [doc, gid, key, masterKey])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc) return
        if (event.change.font_info || event.change.masters)
          setFontGeneration((value) => value + 1)
        const touched =
          event.change.glyph_order ||
          event.change.masters ||
          (gid !== undefined && event.change.glyphs.includes(gid))
        if (touched) setReload((value) => value + 1)
      }),
    [doc, gid],
  )

  if (gid === undefined)
    return {
      detail: null,
      cell: null,
      error: null,
      loading: false,
      fontGeneration,
    }
  const current = loaded.key === key ? loaded : null
  return {
    detail: current?.detail ?? null,
    cell: current?.cell ?? null,
    error: current?.error ?? null,
    loading: current === null,
    fontGeneration,
  }
}
