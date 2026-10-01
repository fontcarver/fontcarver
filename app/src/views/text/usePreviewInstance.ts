/**
 * The preview strip's instance picker and the outlines a chosen instance shows.
 *
 * A master choice is handled by the ordinary text run, which re-shapes with that
 * master. A named instance has no master layer, so the strip renders the
 * interpolated outlines `text.variable_preview` returns for that instance, with
 * the tab's own features, script, language and direction.
 */

import { useEffect, useMemo, useState } from 'react'

import { call, type DocId, type DocSummary } from '../../api'
import type { TextTabState } from '../../stores/uiStore'
import { kerningFeatures } from './kerningModes'
import type {
  PreviewGlyph,
  PreviewInstanceOption,
  PreviewStatus,
} from './presentationTypes'
import { previewInstanceOptions } from './previewSettings'
import { useVariableProof } from './useVariableProof'

/** Named instances of one document, refreshed with its generation. */
export function useProofInstances(
  doc: DocId,
  enabled: boolean,
  generation: number,
): { name: string }[] {
  const [loaded, setLoaded] = useState<{
    key: string
    instances: { name: string }[]
  }>({ key: '', instances: [] })
  const key = `${doc}:${generation}`
  useEffect(() => {
    if (!enabled) return
    let cancelled = false
    void call('variation.state', { doc }).then(
      (state) => {
        if (!cancelled)
          setLoaded({
            key,
            instances: state.instances.map((instance) => ({
              name: instance.name,
            })),
          })
      },
      () => {
        // A design space that cannot be read leaves the picker at its masters.
        if (!cancelled) setLoaded({ key, instances: [] })
      },
    )
    return () => {
      cancelled = true
    }
  }, [doc, enabled, key])
  return enabled && loaded.key === key ? loaded.instances : []
}

/** What the strip renders, and the entries its picker offers. */
export interface PreviewInstanceResult {
  glyphs: readonly PreviewGlyph[]
  activeGlyph: PreviewGlyph | null | undefined
  /** Set only while a named instance is chosen; otherwise the document status. */
  status: PreviewStatus | null
  instances: PreviewInstanceOption[]
}

/** Resolves the picker and, for a named instance, its interpolated outlines. */
export function usePreviewInstance({
  summary,
  state,
  generation,
  documentGlyphs,
  documentActiveGlyph,
}: {
  summary: DocSummary
  state: TextTabState
  generation: number
  documentGlyphs: readonly PreviewGlyph[]
  documentActiveGlyph: PreviewGlyph | null | undefined
}): PreviewInstanceResult {
  const named = useProofInstances(
    summary.doc,
    summary.axes.length > 0,
    generation,
  )
  const instances = useMemo(
    () => previewInstanceOptions(summary.masters, named),
    [named, summary.masters],
  )
  const chosen =
    state.previewInstance === null || state.previewInstance === undefined
      ? null
      : state.previewInstance
  const valid = chosen !== null && chosen < named.length ? chosen : null
  const features = useMemo(
    () => kerningFeatures(state.features, state.kerningMode ?? 'on'),
    [state.features, state.kerningMode],
  )
  const { proof, error } = useVariableProof(
    valid === null
      ? null
      : {
          doc: summary.doc,
          text: state.text,
          location: {},
          instance: valid,
          features,
          direction: state.direction,
          script: state.script ?? null,
          language: state.language ?? null,
        },
    generation,
  )
  const glyphs = useMemo(
    () =>
      (proof?.glyphs ?? []).map((glyph) => ({
        path: glyph.path,
        x: glyph.x,
        y: glyph.y,
        xAdvance: glyph.x_advance,
        yAdvance: glyph.y_advance,
      })),
    [proof],
  )
  if (valid === null)
    return {
      glyphs: documentGlyphs,
      activeGlyph: documentActiveGlyph,
      status: null,
      instances,
    }
  return {
    glyphs,
    activeGlyph:
      state.activeIndex === null ? null : (glyphs[state.activeIndex] ?? null),
    status: error ? 'compile-failed' : proof ? 'ready' : 'stale',
    instances,
  }
}
