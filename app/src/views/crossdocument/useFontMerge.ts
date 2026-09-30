import { useState } from 'react'

import {
  call,
  type DocId,
  type FontCompareResult,
  type FontMergeParams,
  type FontMergeResult,
  type MergeAdoptions,
  type MergeChoice,
} from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import type { GlyphResolution } from './ComparisonResults'

const NO_ADOPTIONS: MergeAdoptions = {
  metadata: false,
  master_metrics: false,
  kerning: false,
}

/** Per-conflict decisions and the reviewed, revision-guarded merge lifecycle. */
export function useFontMerge(input: {
  doc: DocId
  sourceDoc: DocId | null
  targetMaster: string
  sourceMaster: string
  result: FontCompareResult | null
  onFailure: (reason: unknown) => void
  onBusy: (busy: boolean) => void
}) {
  const [choices, setChoices] = useState<Map<string, MergeChoice>>(new Map())
  const [adopt, setAdopt] = useState<MergeAdoptions>(NO_ADOPTIONS)
  const [missingBases, setMissingBases] =
    useState<FontMergeParams['missing_bases']>('copy')
  const [merge, setMerge] = useState<{
    params: FontMergeParams
    result: FontMergeResult
  } | null>(null)

  const reset = () => {
    setChoices(new Map())
    setMerge(null)
  }
  const params = (): FontMergeParams | null => {
    if (!input.result || input.sourceDoc === null) return null
    return {
      doc: input.doc,
      source_doc: input.sourceDoc,
      glyphs: [...choices].map(([name, choice]) => ({ name, choice })),
      adopt,
      target_master: input.targetMaster || null,
      source_master: input.sourceMaster || null,
      missing_bases: missingBases,
      dry_run: true,
      expected_generation: input.result.generation,
      expected_source_generation: input.result.source_generation,
    }
  }
  return {
    choices,
    adopt,
    missingBases,
    merge,
    reset,
    /** True when neither a glyph nor a font-level resource has been chosen. */
    empty:
      choices.size === 0 &&
      !adopt.metadata &&
      !adopt.master_metrics &&
      !adopt.kerning,
    resolve: (name: string, choice: GlyphResolution) => {
      setChoices((current) => {
        const next = new Map(current)
        if (choice === 'skip') next.delete(name)
        else next.set(name, choice)
        return next
      })
      setMerge(null)
    },
    setAdopt: (next: MergeAdoptions) => {
      setAdopt(next)
      setMerge(null)
    },
    setMissingBases: (policy: FontMergeParams['missing_bases']) => {
      setMissingBases(policy)
      setMerge(null)
    },
    preview: async () => {
      const request = params()
      if (!request) return
      input.onBusy(true)
      try {
        setMerge({ params: request, result: await call('font.merge', request) })
      } catch (reason) {
        setMerge(null)
        input.onFailure(reason)
      } finally {
        input.onBusy(false)
      }
    },
    /** Applies the previewed merge as one transaction; resolves to its result. */
    apply: async (): Promise<FontMergeResult | null> => {
      if (!merge) return null
      input.onBusy(true)
      try {
        const applied = await withSettledDocument(merge.params.source_doc, () =>
          withSettledDocument(input.doc, () =>
            call('font.merge', {
              ...merge.params,
              dry_run: false,
              expected_generation: merge.result.generation,
              expected_source_generation: merge.result.source_generation,
            }),
          ),
        )
        reset()
        return applied
      } catch (reason) {
        setMerge(null)
        input.onFailure(reason)
        return null
      } finally {
        input.onBusy(false)
      }
    },
  }
}
