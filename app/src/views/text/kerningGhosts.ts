import { call, type DocId, type GlyphCell, type MasterId } from '../../api'
import type { TextKerningModel } from './kerningModel'

/** Up to four distinct alternatives per side keep visual comparisons legible. */
export const MAX_GHOST_MEMBERS_PER_SIDE = 4

/** A member outline belongs to the first or second glyph of the selected pair. */
export interface KerningGhostMember {
  name: string
  side: 0 | 1
}

/** Takes a bounded subset of the already generation-checked group preview. */
export function ghostMembers(model: TextKerningModel): KerningGhostMember[] {
  const resolution = model.resolution
  if (!resolution) return []
  const scopes = model.scope.split('_')
  return ([0, 1] as const).flatMap((side) => {
    if (scopes[side] !== 'group') return []
    const group = side === 0 ? resolution.left_group : resolution.right_group
    const actual = side === 0 ? model.pair.left : model.pair.right
    // loadKerningImpact preserves requested side order. Names alone cannot
    // identify sides: independent side1/side2 groups may share the same name.
    const impact = model.impact[side === 1 && scopes[0] === 'group' ? 1 : 0]
    const members = impact?.name === group ? impact.members : []
    return [...new Set(members)]
      .filter((name) => name !== actual)
      .slice(0, MAX_GHOST_MEMBERS_PER_SIDE)
      .map((name) => ({ name, side }))
  })
}

/** Exact session handles and active-master outlines; no font-wide transfer. */
export async function loadKerningGhostCells(
  doc: DocId,
  master: MasterId,
  names: readonly string[],
  signal: AbortSignal,
): Promise<ReadonlyMap<string, GlyphCell>> {
  const wanted = [...new Set(names)].slice(0, MAX_GHOST_MEMBERS_PER_SIDE * 2)
  const cells = new Map<string, GlyphCell>()
  let next = 0
  const worker = async () => {
    while (next < wanted.length) {
      signal.throwIfAborted()
      const name = wanted[next++]!
      const matches = await call('glyphs.search', {
        doc,
        query: name,
        limit: 32,
      })
      signal.throwIfAborted()
      const exact = matches.find((cell) => cell.name === name)
      if (!exact) continue
      const cell = await call('glyphs.cell', { doc, master, gid: exact.gid })
      signal.throwIfAborted()
      if (cell.name === name && cell.path) cells.set(name, cell)
    }
  }
  await Promise.all(Array.from({ length: Math.min(2, wanted.length) }, worker))
  return cells
}
