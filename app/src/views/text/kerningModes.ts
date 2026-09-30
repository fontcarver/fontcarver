import type { DocId } from '../../api'
import { selectActiveTab, useUiStore, type UiState } from '../../stores/uiStore'

/** Kerning shaping/editing policy for one text tab. */
export type KerningMode = 'off' | 'on' | 'locked'

/** Off overrides shaping without destroying the tab's stored feature choices. */
export function kerningFeatures(
  features: Array<[string, boolean]>,
  mode: KerningMode,
): Array<[string, boolean]> {
  if (mode !== 'off') return features
  const next = features.map(([tag, enabled]): [string, boolean] => [
    tag,
    tag === 'kern' || tag === 'vkrn' ? false : enabled,
  ])
  for (const tag of ['kern', 'vkrn'])
    if (!next.some(([key]) => key === tag)) next.push([tag, false])
  return next
}

function isSpacingLocked(state: UiState, doc: DocId): boolean {
  const tab = selectActiveTab(state)
  return tab?.doc === doc && tab.textState?.kerningMode === 'locked'
}

/** Reads the current guard at command time, including during queued callbacks. */
export function spacingLockedForDocument(doc: DocId): boolean {
  return isSpacingLocked(useUiStore.getState(), doc)
}

/** Keeps metric controls in sync with the active text tab's spacing guard. */
export function useSpacingLocked(doc: DocId): boolean {
  return useUiStore((state) => isSpacingLocked(state, doc))
}
