import { useCallback, useState } from 'react'
import type { KerningResolution } from '../../api'
import { defaultKerningScope, type KerningScope } from './kerningModel'

interface Choice {
  key: string
  scope: KerningScope
  /** The user picked this scope in the panel. */
  explicit: boolean
  /** An automatic answer has been taken for this pair already. */
  settled: boolean
}

/**
 * The scope the kerning panel is on, and how it got there.
 *
 * The scope is decided **once** per pair, from the rule that applies when the
 * pair is opened, and then holds until the pair changes or the user picks
 * another scope. Recomputing it on every reload would make the panel move
 * itself: a held-modifier exception creates a glyph-scoped rule, and the panel
 * would then quietly adopt that scope for the ordinary keys too, which is
 * exactly the persistence row 41 asks for. Waiting for the first resolution
 * before settling is what keeps the pair from opening on a stale default.
 */
export function useKerningScope(
  key: string,
  resolution: KerningResolution | null,
) {
  const [choice, setChoice] = useState<Choice>({
    key,
    scope: 'glyph_glyph',
    explicit: false,
    settled: false,
  })
  const automatic = resolution ? defaultKerningScope(resolution) : null
  let next = choice
  if (choice.key !== key)
    next = {
      key,
      scope: automatic ?? 'glyph_glyph',
      explicit: false,
      settled: automatic !== null,
    }
  else if (!choice.explicit && !choice.settled && automatic !== null)
    next = { ...choice, scope: automatic, settled: true }
  if (next !== choice) setChoice(next)

  const select = useCallback(
    (scope: KerningScope) => {
      setChoice({ key, scope, explicit: true, settled: true })
    },
    [key],
  )
  return [next.scope, select] as const
}
