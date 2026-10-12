/**
 * The grid and keyboard increments this font asks for.
 *
 * Grid spacing, its subdivision and the arrow-key increments belong to the
 * drawing rather than to the person using the application: a pixel font needs a
 * grid of two whoever opens it. The project stores them per font and the kernel
 * hands back the values to actually use, already resolved against the
 * application preference, so the editor asks once per document and lets
 * `prefsStore` apply the result.
 *
 * A font that stores nothing, and a read that fails, both leave the preference
 * in charge: an optional setting must never stop a glyph being edited.
 */

import { useEffect, useState } from 'react'

import { call, type DocId, type EditorSettingsView } from '../../api'

/** The resolved values, or `null` while unknown or unavailable. */
export type ResolvedEditorSettings = Pick<
  EditorSettingsView,
  'grid_size' | 'grid_subdivision' | 'nudge'
> | null

/** Reads one document's resolved editor settings, refetching per generation. */
export function useFontEditorSettings(
  doc: DocId | null,
  generation: number,
): ResolvedEditorSettings {
  // The answer is stored under the document and generation it came from, so a
  // document that has not answered yet reports nothing rather than another
  // font's grid, and no render has to clear it.
  const key = `${doc ?? 'none'}:${generation}`
  const [loaded, setLoaded] = useState<{
    key: string
    value: ResolvedEditorSettings
  } | null>(null)
  useEffect(() => {
    if (doc === null) return
    let current = true
    void call('fontinfo.editor_settings_get', { doc })
      .then((view) => {
        if (!current) return
        setLoaded({
          key,
          value: {
            grid_size: view.grid_size,
            grid_subdivision: view.grid_subdivision,
            nudge: view.nudge,
          },
        })
      })
      .catch(() => {
        if (current) setLoaded({ key, value: null })
      })
    return () => {
      current = false
    }
  }, [doc, key])
  return loaded?.key === key ? loaded.value : null
}
