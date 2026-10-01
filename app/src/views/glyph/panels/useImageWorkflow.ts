import { useCallback, useEffect, useRef, useState } from 'react'
import {
  call,
  onDocumentEvent,
  type Image,
  type ImageState,
  type ImageTraceResult,
} from '../../../api'
import { activeEditor } from '../../../editor/activeEditor'
import {
  holdDocumentEdits,
  noteDocumentEdit,
  pendingDocumentEdits,
} from '../../../editor/editSession'
import type { LayerTarget } from '../../../editor/pointCommands'
import { rawMessage } from '../../../i18n/errors'
import { useEditorStore } from '../../../stores/editorStore'

/** Trace settings retained unchanged between review and application. */
export interface ImageTraceSettings {
  threshold: number
  tolerance: number
  /** Fit cubic curves; false simplifies to straight lines only. */
  curves: boolean
  /** Turn angle in degrees at or above which a vertex stays a corner. */
  corner_angle: number
  /** Low-pass radius applied before fitting, in source pixels. */
  smoothing: number
  /** Smallest retained ring area in square source pixels. */
  min_area: number
  append: boolean
}

/** The settings a freshly opened section starts from. */
export const DEFAULT_TRACE_SETTINGS: ImageTraceSettings = {
  threshold: 128,
  tolerance: 1,
  curves: true,
  corner_angle: 35,
  smoothing: 0,
  // A scan carries dust: four square pixels drops a speck without touching any
  // mark a designer drew. Zero remains reachable, and keeps every ring.
  min_area: 4,
  append: true,
}

/** Waits for gestures and publishes the mutation before save or navigation can overtake it. */
function mutate<T>(doc: number, work: () => Promise<T>): Promise<T> {
  const settling = activeEditor()?.settle()
  const earlier = pendingDocumentEdits(doc)
  noteDocumentEdit(doc)
  const pending = Promise.all([settling, earlier]).then(work)
  holdDocumentEdits(doc, pending)
  return pending
}

/** Keeps source-image controls and trace review tied to the loaded document revision. */
export function useImageWorkflow(target: LayerTarget, reload: () => void) {
  const { doc, glyph, layer } = target
  const [revision, setRevision] = useState(0)
  const editing = useEditorStore((store) => store.editing)
  const deferred = useRef(false)
  const reviewEpoch = useRef(0)
  const identity = JSON.stringify([doc, glyph, layer, revision])
  const [answer, setAnswer] = useState<{
    identity: string
    state: ImageState | null
    error: string | null
  } | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [review, setReview] = useState<{
    identity: string
    settings: ImageTraceSettings
    result: ImageTraceResult
  } | null>(null)
  const latest = useRef({ identity, mounted: true })
  useEffect(() => {
    latest.current = { identity, mounted: true }
    return () => {
      latest.current.mounted = false
    }
  }, [identity])
  const refresh = useCallback(() => {
    reviewEpoch.current += 1
    setRevision((value) => value + 1)
    setReview(null)
  }, [])
  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc !== doc) return
        if (useEditorStore.getState().editing) {
          reviewEpoch.current += 1
          deferred.current = true
          setReview(null)
          return
        }
        refresh()
      }),
    [doc, refresh],
  )
  useEffect(() => {
    if (editing || !deferred.current) return
    deferred.current = false
    refresh()
  }, [editing, refresh])
  useEffect(() => {
    let cancelled = false
    void call('image.state', { doc, glyph, layer }).then(
      (state) => {
        if (!cancelled) setAnswer({ identity, state, error: null })
      },
      (error) => {
        if (!cancelled)
          setAnswer({ identity, state: null, error: rawMessage(error) })
      },
    )
    return () => {
      cancelled = true
    }
  }, [doc, glyph, layer, identity])
  const current = answer?.identity === identity ? answer : null
  const state = current?.state ?? null
  const preview = review?.identity === identity ? review : null

  const run = async (work: () => Promise<unknown>, writes: boolean) => {
    if (busy) return
    setBusy(true)
    setError(null)
    setReview(null)
    try {
      if (writes) await mutate(doc, work)
      else {
        await activeEditor()?.settle()
        await pendingDocumentEdits(doc)
        await work()
      }
      if (writes) {
        refresh()
        reload()
      }
    } catch (error) {
      if (latest.current.mounted) setError(rawMessage(error))
      refresh()
    } finally {
      if (latest.current.mounted) setBusy(false)
    }
  }
  const setImage = (image: Image | null) => {
    if (!state) return
    void run(
      () =>
        call('image.set', {
          ...target,
          expected_generation: state.generation,
          image,
        }),
      true,
    )
  }
  const importImage = (file: {
    path: string | null
    data: number[] | null
    file_name: string
    embed: boolean
  }) => {
    if (!state) return
    void run(
      () =>
        call('image.import', {
          ...target,
          expected_generation: state.generation,
          ...file,
        }),
      true,
    )
  }
  const importFile = (file: File) => {
    if (!state) return
    void run(async () => {
      const data = Array.from(new Uint8Array(await file.arrayBuffer()))
      await call('image.import', {
        ...target,
        expected_generation: state.generation,
        path: null,
        data,
        file_name: file.name,
        embed: true,
      })
    }, true)
  }
  const previewTrace = (settings: ImageTraceSettings) => {
    if (!state) return
    const epoch = reviewEpoch.current
    void run(async () => {
      const result = await call('image.trace', {
        ...target,
        ...settings,
        expected_generation: state.generation,
        expected_source_hash: null,
        dry_run: true,
      })
      if (
        latest.current.mounted &&
        latest.current.identity === identity &&
        reviewEpoch.current === epoch
      )
        setReview({ identity, settings, result })
    }, false)
  }
  const applyTrace = () => {
    if (!preview) return
    void run(
      () =>
        call('image.trace', {
          ...target,
          ...preview.settings,
          expected_generation: preview.result.generation,
          expected_source_hash: preview.result.source_hash,
          dry_run: false,
        }),
      true,
    )
  }
  return {
    state,
    busy,
    editing,
    error: error ?? current?.error ?? null,
    loading: current === null,
    preview,
    refresh,
    setImage,
    importImage,
    importFile,
    previewTrace,
    applyTrace,
    clearPreview: () => {
      reviewEpoch.current += 1
      setReview(null)
    },
  }
}
