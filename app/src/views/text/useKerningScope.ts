import { useCallback, useState } from 'react'
import type { KerningResolution } from '../../api'
import { defaultKerningScope, type KerningScope } from './kerningModel'

/** Retains the selected scope during query reloads and resets it for new pairs. */
export function useKerningScope(
  key: string,
  resolution: KerningResolution | null,
) {
  const [choice, setChoice] = useState({
    key,
    scope: 'glyph_glyph' as KerningScope,
    explicit: false,
  })
  const automatic = resolution ? defaultKerningScope(resolution) : null
  const scope =
    choice.key === key
      ? choice.explicit
        ? choice.scope
        : (automatic ?? choice.scope)
      : (automatic ?? 'glyph_glyph')
  // Keep the last resolved automatic scope while this same pair reloads, so
  // repeats still find the captured class run instead of switching to glyphs.
  if (choice.key !== key || choice.scope !== scope) {
    setChoice({ key, scope, explicit: false })
  }
  const select = useCallback(
    (scope: KerningScope) => {
      setChoice({ key, scope, explicit: true })
    },
    [key],
  )
  return [scope, select] as const
}
