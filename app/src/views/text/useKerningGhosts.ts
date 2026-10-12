import { useEffect, useMemo, useState } from 'react'
import type { DocId, GlyphCell, MasterId } from '../../api'
import { ghostMembers, loadKerningGhostCells } from './kerningGhosts'
import type { TextKerningModel } from './kerningModel'
import type { TextKerningGhost } from './textCanvas'
import type { PositionedGlyph } from './textLayout'

interface Options {
  doc: DocId
  master: MasterId | null
  generation: number
  model: TextKerningModel
  glyphs: readonly PositionedGlyph[]
  selectedGap: number | null
  pending: boolean
  direction: string | null
}

/** Accessible controls and canvas data share one current selection identity. */
export interface KerningGhostModel {
  enabled: boolean
  available: boolean
  loading: boolean
  failed: boolean
  outlines: readonly TextKerningGhost[]
  value: number | null
  setEnabled: (enabled: boolean) => void
}

/** Loads ghost outlines on demand and hides old results synchronously on change. */
export function useKerningGhosts(options: Options): KerningGhostModel {
  const { doc, master, generation, model, selectedGap, pending, direction } =
    options
  const [enabled, setEnabled] = useState(false)
  const [loaded, setLoaded] = useState<{
    key: string
    cells: ReadonlyMap<string, GlyphCell>
    failed: boolean
  } | null>(null)
  const left = selectedGap === null ? undefined : options.glyphs[selectedGap]
  const right =
    selectedGap === null ? undefined : options.glyphs[selectedGap + 1]
  const current =
    !pending &&
    !model.loading &&
    !model.loadFailed &&
    model.resolution?.status.generation === generation &&
    model.resolution.left === model.pair.left &&
    model.resolution.right === model.pair.right &&
    left?.name === model.pair.left &&
    right?.name === model.pair.right
  const value =
    current && Number.isFinite(model.resolution?.value)
      ? (model.resolution?.value ?? null)
      : null
  const memberKey = JSON.stringify(
    current && !model.impactLoading && !model.impactFailed
      ? ghostMembers(model)
      : [],
  )
  const members = useMemo(
    () => JSON.parse(memberKey) as ReturnType<typeof ghostMembers>,
    [memberKey],
  )
  const key = JSON.stringify([
    doc,
    master,
    generation,
    direction,
    selectedGap,
    model.pair,
    memberKey,
  ])
  const available = current && master !== null && members.length > 0
  const requested = enabled && available
  useEffect(() => {
    if (!requested || master === null) return
    const abort = new AbortController()
    void loadKerningGhostCells(
      doc,
      master,
      members.map((item) => item.name),
      abort.signal,
    )
      .then((cells) => {
        if (!abort.signal.aborted) setLoaded({ key, cells, failed: false })
      })
      .catch(() => {
        if (!abort.signal.aborted) {
          abort.abort()
          setLoaded({ key, cells: new Map(), failed: true })
        }
      })
    return () => abort.abort()
  }, [doc, key, master, members, requested])
  const ready = requested && loaded?.key === key
  const outlines = useMemo(() => {
    if (!ready || !loaded || !left || !right || selectedGap === null) return []
    return members.flatMap(({ name, side }): TextKerningGhost[] => {
      const cell = loaded.cells.get(name)
      if (!cell?.path) return []
      const glyph = side === 0 ? left : right
      // Vertical placement honors the member's own origin. Horizontal members
      // share the selected side's baseline and origin, without invented shaping.
      const vertical = direction === 'ttb' && glyph.penY !== undefined
      return [
        {
          index: selectedGap + side,
          name,
          path: cell.path,
          x: vertical ? (glyph.penX ?? 0) - cell.width / 2 : glyph.x,
          y: vertical
            ? glyph.penY! - (cell.vertical_origin ?? glyph.penY! - glyph.y)
            : glyph.y,
        },
      ]
    })
  }, [direction, left, loaded, members, ready, right, selectedGap])
  return {
    enabled,
    available,
    loading: requested && !ready,
    failed: ready && loaded.failed,
    outlines,
    value,
    setEnabled,
  }
}
