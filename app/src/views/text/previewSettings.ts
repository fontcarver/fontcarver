/**
 * Translation between the tab state and the preview strip's settings.
 *
 * The strip is fully controlled so the docked strip, the detached preview window
 * and a saved workspace all show the same waterfall; this module is the single
 * place that knows the defaults and how a picker choice becomes tab state.
 */

import type { TextTabState } from '../../stores/uiStore'
import type {
  PreviewInstanceOption,
  PreviewSettings,
} from './presentationTypes'
import { PREVIEW_SIZES } from './previewPaint'

const MASTER_PREFIX = 'master:'
const INSTANCE_PREFIX = 'instance:'

/** The picker value for the master or named instance the tab is previewing. */
export function previewInstanceId(state: TextTabState): string {
  return state.previewInstance === null || state.previewInstance === undefined
    ? `${MASTER_PREFIX}${state.master ?? ''}`
    : `${INSTANCE_PREFIX}${state.previewInstance}`
}

/** Reads the strip's settings from one tab, filling in the shipped defaults. */
export function previewSettingsOf(state: TextTabState): PreviewSettings {
  return {
    flipHorizontal: state.previewFlipHorizontal === true,
    flipVertical: state.previewFlipVertical === true,
    blur:
      typeof state.previewBlur === 'number' &&
      Number.isFinite(state.previewBlur)
        ? Math.min(4, Math.max(0, state.previewBlur))
        : 0,
    mode: state.previewMode === 'single' ? 'single' : 'run',
    sizes:
      state.previewSizes && state.previewSizes.length > 0
        ? state.previewSizes
        : [...PREVIEW_SIZES],
    instance: previewInstanceId(state),
  }
}

/**
 * The tab-state patch one settings change makes.
 *
 * A master choice moves the whole text run to that master, because previewing a
 * master means shaping with it; a named instance only changes what the strip
 * renders, so the edited master is left alone.
 */
export function previewSettingsPatch(
  changes: Partial<PreviewSettings>,
): Partial<TextTabState> {
  const patch: { -readonly [K in keyof TextTabState]?: TextTabState[K] } = {}
  if (changes.flipHorizontal !== undefined)
    patch.previewFlipHorizontal = changes.flipHorizontal
  if (changes.flipVertical !== undefined)
    patch.previewFlipVertical = changes.flipVertical
  if (changes.blur !== undefined)
    patch.previewBlur = Math.min(4, Math.max(0, changes.blur))
  if (changes.mode !== undefined) patch.previewMode = changes.mode
  if (changes.sizes !== undefined) patch.previewSizes = [...changes.sizes]
  if (changes.instance !== undefined) {
    if (changes.instance.startsWith(INSTANCE_PREFIX)) {
      const index = Number(changes.instance.slice(INSTANCE_PREFIX.length))
      if (Number.isSafeInteger(index) && index >= 0)
        patch.previewInstance = index
    } else if (changes.instance.startsWith(MASTER_PREFIX)) {
      patch.previewInstance = null
      const master = changes.instance.slice(MASTER_PREFIX.length)
      if (master !== '') patch.master = master
    }
  }
  return patch
}

/** The picker's entries: every master first, then every named instance. */
export function previewInstanceOptions(
  masters: readonly { id: string; name: string }[],
  instances: readonly { name: string }[],
): PreviewInstanceOption[] {
  return [
    ...masters.map((master) => ({
      id: `${MASTER_PREFIX}${master.id}`,
      name: master.name,
      kind: 'master' as const,
    })),
    ...instances.map((instance, index) => ({
      id: `${INSTANCE_PREFIX}${index}`,
      name: instance.name,
      kind: 'instance' as const,
    })),
  ]
}
