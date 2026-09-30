import { useEffect, useMemo, useRef, useState } from 'react'

import { call, onDocumentEvent, type Image } from '../../api'
import type { SourceImageDraw } from '../../canvas/sourceImage'
import type { LayerTarget } from '../../editor/pointCommands'
import { useEditorStore } from '../../stores/editorStore'

/** Loads a layer's resolved PNG, releasing browser resources after decoding. */
export function useSourceImage(
  target: LayerTarget | null,
  image: Image | null,
  refreshKey?: unknown,
): SourceImageDraw | null {
  const doc = target?.doc
  const glyph = target?.glyph
  const layer = target?.layer
  // Outline refreshes can return new objects with unchanged image contents.
  const sourceKey = useMemo(() => JSON.stringify(image), [image])
  const identity =
    target !== null && image !== null
      ? JSON.stringify([doc, glyph, layer, sourceKey])
      : ''
  const [generation, setGeneration] = useState(0)
  const editing = useEditorStore((store) => store.editing)
  const deferred = useRef(false)
  const [loaded, setLoaded] = useState<{
    identity: string
    draw: SourceImageDraw | null
  }>({ identity: '', draw: null })

  useEffect(() => {
    if (doc === undefined) return
    return onDocumentEvent((event) => {
      // Refreshing a linked file can leave the layer metadata unchanged.
      if (event.doc !== doc) return
      if (useEditorStore.getState().editing) {
        deferred.current = true
        return
      }
      setGeneration((value) => value + 1)
    })
  }, [doc])

  useEffect(() => {
    // A point drag emits every frame; decode its source only after it ends.
    if (editing || !deferred.current) return
    deferred.current = false
    setGeneration((value) => value + 1)
  }, [editing])

  useEffect(() => {
    if (
      identity === '' ||
      doc === undefined ||
      glyph === undefined ||
      layer === undefined
    ) {
      return
    }
    let cancelled = false
    let url: string | null = null
    let bitmap: HTMLImageElement | null = null
    const release = () => {
      if (bitmap !== null) {
        bitmap.onload = null
        bitmap.onerror = null
      }
      if (url !== null) {
        URL.revokeObjectURL(url)
        url = null
      }
    }
    void call('image.state', { doc, glyph, layer })
      .then((result) => {
        if (cancelled) return
        const { image: resolved, png, width, height, error } = result
        if (
          error !== null ||
          resolved === null ||
          png === null ||
          width === null ||
          height === null ||
          width <= 0 ||
          height <= 0
        ) {
          setLoaded({ identity, draw: null })
          return
        }
        url = URL.createObjectURL(
          new Blob([new Uint8Array(png)], { type: 'image/png' }),
        )
        const decoded = new window.Image()
        bitmap = decoded
        decoded.onload = () => {
          release()
          if (cancelled) return
          setLoaded({
            identity,
            draw: {
              bitmap: decoded,
              width,
              height,
              transform: resolved.transform,
            },
          })
        }
        decoded.onerror = () => {
          release()
          if (!cancelled) setLoaded({ identity, draw: null })
        }
        decoded.src = url
      })
      .catch(() => {
        release()
        if (!cancelled) setLoaded({ identity, draw: null })
      })
    return () => {
      cancelled = true
      release()
    }
  }, [doc, glyph, layer, identity, generation, refreshKey])

  // A target/source change hides the old bitmap before the next effect runs.
  return identity !== '' && loaded.identity === identity ? loaded.draw : null
}
