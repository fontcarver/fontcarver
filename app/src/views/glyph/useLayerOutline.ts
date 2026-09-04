/**
 * Loads the editable outline of one layer.
 *
 * `glyphs.detail` carries the persistent glyph and its flattened preview
 * paths; `glyphs.outline` carries the *editable* layer — stored contours in
 * stored order, resolved components, anchors, guides and metrics — which is
 * what every editing command addresses. The editor draws from this response so
 * the geometry on screen and the geometry the commands index are the same.
 *
 * Document events are deliberately ignored while a gesture is live: an
 * `edit_apply` emits one per animation frame, and refetching mid-drag would
 * fight the optimistic preview and stutter the outline. The reload that was
 * skipped is taken as soon as the gesture ends.
 */

import { useCallback, useEffect, useRef, useState } from 'react'

import {
  call,
  onDocumentEvent,
  type DocId,
  type GlyphId,
  type LayerId,
  type LayerOutline,
} from '../../api'
import { useEditorStore } from '../../stores/editorStore'

/** What `useLayerOutline` reports. */
export interface LayerOutlineResult {
  outline: LayerOutline | null
  error: string | null
  /** Refetches now; called when a gesture finishes. */
  reload: () => void
}

/** Fetches and keeps fresh the editable outline of one layer. */
export function useLayerOutline(
  doc: DocId,
  gid: GlyphId | undefined,
  layer: LayerId | null,
): LayerOutlineResult {
  const [generation, setGeneration] = useState(0)
  const [state, setState] = useState<{
    key: string
    outline: LayerOutline | null
    error: string | null
  }>({ key: '', outline: null, error: null })
  const editing = useEditorStore((store) => store.editing)
  const deferred = useRef(false)

  const layerKey = layer ?? ''
  const key = `${doc}:${gid ?? 'none'}:${layerKey}:${generation}`

  const reload = useCallback(() => setGeneration((value) => value + 1), [])

  useEffect(() => {
    if (gid === undefined) return
    let cancelled = false
    void call('glyphs.outline', {
      doc,
      gid,
      layer: layerKey === '' ? null : layerKey,
    })
      .then((outline) => {
        if (!cancelled) setState({ key, outline, error: null })
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setState({
          key,
          outline: null,
          error: error instanceof Error ? error.message : String(error),
        })
      })
    return () => {
      cancelled = true
    }
  }, [doc, gid, key, layerKey])

  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc || gid === undefined) return
        if (!event.change.glyphs.includes(gid) && !event.change.layers) return
        if (useEditorStore.getState().editing) {
          deferred.current = true
          return
        }
        reload()
      }),
    [doc, gid, reload],
  )

  // A gesture that ended while events were being ignored still owes one load.
  useEffect(() => {
    if (editing || !deferred.current) return
    deferred.current = false
    reload()
  }, [editing, reload])

  const current = state.key === key ? state : null
  return {
    outline: current?.outline ?? null,
    error: current?.error ?? null,
    reload,
  }
}
