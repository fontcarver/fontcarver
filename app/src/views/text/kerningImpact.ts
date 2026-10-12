import { useEffect, useState } from 'react'
import {
  call,
  type DocId,
  type KerningResolution,
  type KerningGroupSide,
} from '../../api'
import { kerningScopeSides, type KerningScope } from './kerningModel'

/** A bounded preview of one group's affected glyphs. */
export interface KerningGroupImpact {
  name: string
  total: number
  members: string[]
}

/** Reads at most eight members per relevant group at the resolution generation. */
export async function loadKerningImpact(
  doc: DocId,
  resolution: KerningResolution,
  scope: KerningScope,
): Promise<KerningGroupImpact[]> {
  const sides = kerningScopeSides(resolution, scope)
  if (!sides) return []
  const groups: { side: KerningGroupSide; name: string }[] = []
  if ('group' in sides.left)
    groups.push({ side: 'side1', name: sides.left.group })
  if ('group' in sides.right)
    groups.push({ side: 'side2', name: sides.right.group })
  return Promise.all(
    groups.map(async (group) => {
      const result = await call('kerning.members', {
        doc,
        ...group,
        generation: resolution.status.generation,
        offset: 0,
        limit: 8,
      })
      return { name: group.name, total: result.total, members: result.items }
    }),
  )
}

/** Blocks group edits until a preview from the same generation has arrived. */
export function useKerningImpact(
  doc: DocId,
  resolution: KerningResolution | null,
  scope: KerningScope,
) {
  const key = JSON.stringify([
    doc,
    resolution?.status.generation,
    resolution?.left_group,
    resolution?.right_group,
    scope,
  ])
  const [loaded, setLoaded] = useState<{
    key: string
    groups: KerningGroupImpact[]
    failed: boolean
  } | null>(null)
  const required = scope !== 'glyph_glyph' && !!resolution
  useEffect(() => {
    if (!required || !resolution) return
    let current = true
    void loadKerningImpact(doc, resolution, scope)
      .then((groups) => {
        if (current) setLoaded({ key, groups, failed: false })
      })
      .catch(() => {
        if (current) setLoaded({ key, groups: [], failed: true })
      })
    return () => {
      current = false
    }
  }, [doc, key, required, resolution, scope])
  const ready = !required || loaded?.key === key
  return {
    groups: required && loaded?.key === key ? loaded.groups : [],
    loading: !ready,
    failed: required && loaded?.key === key ? loaded.failed : false,
  }
}
