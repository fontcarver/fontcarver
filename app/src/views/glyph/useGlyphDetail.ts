/**
 * Loads everything one glyph tab draws.
 *
 * `glyphs.detail` carries the persistent glyph plus one flattened path per
 * layer; `glyphs.cell` adds the exact outline bounds of the selected master,
 * which is what the sidebearing readouts need. Both are re-requested when a
 * document event touches this glyph — there is no polling.
 *
 * A reload of the *same* glyph keeps showing the data it already has until the
 * new answer arrives. Blanking would unmount the canvas for a frame, and an
 * editing gesture lives in that canvas: every `edit_apply` emits a document
 * event, so a blank-on-reload would tear down the drag the user is still
 * holding. For the same reason reloads are deferred entirely while a gesture
 * is live and taken once it ends.
 */

import { useEffect, useRef, useState } from 'react'

import {
  call,
  onDocumentEvent,
  type DocId,
  type GlyphCell,
  type GlyphDetail,
  type GlyphId,
  type MasterId,
} from '../../api'
import { useEditorStore } from '../../stores/editorStore'

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
  /** Identity of the glyph the data describes, without the reload counter. */
  identity: string
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
    identity: '',
    detail: null,
    cell: null,
    error: null,
  })
  const editing = useEditorStore((state) => state.editing)
  const deferred = useRef(false)

  const masterKey = master ?? ''
  const identity = `${doc}:${gid ?? 'none'}:${masterKey}`
  const key = `${identity}:${reload}`

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
          identity,
          detail: loadedDetail,
          cell: loadedCell,
          error: null,
        })
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setLoaded({
          key,
          identity,
          detail: null,
          cell: null,
          error: error instanceof Error ? error.message : String(error),
        })
      })

    return () => {
      cancelled = true
    }
  }, [doc, gid, identity, key, masterKey])

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
        if (!touched) return
        if (useEditorStore.getState().editing) {
          deferred.current = true
          return
        }
        setReload((value) => value + 1)
      }),
    [doc, gid],
  )

  // A gesture that ended while events were being ignored still owes one load.
  useEffect(() => {
    if (editing || !deferred.current) return
    deferred.current = false
    setReload((value) => value + 1)
  }, [editing])

  if (gid === undefined)
    return {
      detail: null,
      cell: null,
      error: null,
      loading: false,
      fontGeneration,
    }
  // Data for the same glyph stays on screen while a reload is in flight; only
  // a different glyph, master or document blanks the view.
  const current = loaded.identity === identity ? loaded : null
  return {
    detail: current?.detail ?? null,
    cell: current?.cell ?? null,
    error: current?.error ?? null,
    loading: loaded.key !== key,
    fontGeneration,
  }
}
