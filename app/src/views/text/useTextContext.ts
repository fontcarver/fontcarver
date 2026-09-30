import { useMemo } from 'react'
import type { DocSummary } from '../../api'
import type { TextTabState } from '../../stores/uiStore'
import { kerningFeatures } from './kerningModes'
import type { TextRequestContext } from './textLayout'

/** One shaping identity shared by the editor, proof sheets and detached preview. */
export function useTextContext(
  summary: DocSummary,
  state: TextTabState,
  generation: number,
): TextRequestContext {
  return useMemo(
    () => ({
      doc: summary.doc,
      generation,
      unitsPerEm: summary.units_per_em,
      master: state.master,
      text: state.text,
      features: kerningFeatures(state.features, state.kerningMode ?? 'on'),
      direction: state.direction,
      script: state.script ?? null,
      language: state.language ?? null,
    }),
    [
      summary.doc,
      summary.units_per_em,
      generation,
      state.master,
      state.text,
      state.features,
      state.kerningMode,
      state.direction,
      state.script,
      state.language,
    ],
  )
}
