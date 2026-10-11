import { useCallback, useMemo, useState } from 'react'

import type { DocId } from '../../api'
import type { TextCutOffBand, TextSpacingBeam } from './textCanvas'
import type { PositionedGlyph } from './textLayout'
import {
  beamMargins,
  resolveSpacingView,
  runGlyphNames,
  type SpacingViewState,
} from './spacingMatrixModel'
import { useSpacingMatrix } from './useSpacingMatrix'
import { useSpacingViewChoices, useSpacingViewStore } from './spacingView'

/**
 * One source of truth for the Spacing panel and its canvas overlays.
 *
 * The matrix rows and the beam measurements are the same answer, so they are
 * fetched once here: the panel renders the rows, and the canvas is handed the
 * beam and the cut-off band through the run object, which is what makes a
 * change repaint. Rows are fetched while the panel is open or the beam is on,
 * and never otherwise, so a collapsed panel costs nothing.
 *
 * Dragging the beam on the canvas moves the line every pointer move, which must
 * not be a request every pointer move. During a drag the measured height is
 * held at where the drag started and the canvas draws no distances, so the run
 * never shows numbers that belong to a height the beam has already left; the
 * settled height is measured once on release.
 */
export function useSpacingOverlay({
  tab,
  doc,
  master,
  generation,
  unitsPerEm,
  glyphs,
}: {
  tab: string
  doc: DocId
  master: string | null
  generation: number
  unitsPerEm: number
  glyphs: readonly PositionedGlyph[]
}) {
  const [revision, setRevision] = useState(0)
  // While the beam is dragged this holds the height it was grabbed at, which
  // is the height the rows on screen were measured for.
  const [drag, setDrag] = useState<{ height: number } | null>(null)
  const stored = useSpacingViewChoices(tab)
  const update = useSpacingViewStore((state) => state.update)
  const view = resolveSpacingView(stored, unitsPerEm)
  const names = useMemo(() => runGlyphNames(glyphs), [glyphs])
  const dragging = drag !== null
  const measured = drag ? drag.height : view.beamHeight
  const answer = useSpacingMatrix(
    doc,
    master,
    generation,
    names,
    view.beam ? measured : null,
    view.open || view.beam,
    revision,
  )
  const rows = answer?.data?.rows ?? null
  const beam = useMemo<TextSpacingBeam | null>(
    () =>
      view.beam && rows
        ? {
            height: view.beamHeight,
            margins: dragging ? NO_MARGINS : beamMargins(rows),
          }
        : null,
    [dragging, rows, view.beam, view.beamHeight],
  )
  const cutOff = useMemo<TextCutOffBand | null>(
    () => (view.cutOff ? { from: view.cutOffFrom, to: view.cutOffTo } : null),
    [view.cutOff, view.cutOffFrom, view.cutOffTo],
  )
  const onViewChange = useCallback(
    (changes: Partial<SpacingViewState>) => update(tab, changes),
    [tab, update],
  )
  const refresh = useCallback(() => setRevision((value) => value + 1), [])
  const onBeamGrab = useCallback(
    () =>
      setDrag({
        height: resolveSpacingView(
          useSpacingViewStore.getState().byTab[tab],
          unitsPerEm,
        ).beamHeight,
      }),
    [tab, unitsPerEm],
  )
  const onBeamDrag = useCallback(
    (height: number) => {
      if (Number.isFinite(height))
        update(tab, { beamHeight: Math.round(height) })
    },
    [tab, update],
  )
  const onBeamRelease = useCallback(() => setDrag(null), [])
  return {
    view,
    glyphs: names,
    rows,
    loading: names.length > 0 && (view.open || view.beam) && answer === null,
    failed: answer?.failed ?? false,
    beam,
    cutOff,
    dragging,
    onViewChange,
    refresh,
    onBeamGrab,
    onBeamDrag,
    onBeamRelease,
  }
}

const NO_MARGINS: ReadonlyMap<string, { left: number; right: number }> =
  new Map()
