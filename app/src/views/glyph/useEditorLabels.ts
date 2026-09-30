/**
 * Translated strings the editor modules need.
 *
 * The editor is plain TypeScript with no i18n imports, so the view resolves
 * every undo label and readout word once and hands them in. That keeps the
 * interaction code testable with plain strings and keeps `t()` where React can
 * re-run it after a language change.
 */

import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import type { EditorLabels } from '../../editor/editorController'
import type { HudLabels } from '../../editor/hud'

/** Builds the editor's label bundle for the current language. */
export function useEditorLabels(): EditorLabels {
  const { t } = useTranslation()
  const hud = useMemo<HudLabels>(
    () => ({
      deltaX: t('editor.hud.deltaX'),
      deltaY: t('editor.hud.deltaY'),
      angle: t('editor.hud.angle'),
      length: t('editor.hud.length'),
      width: t('editor.hud.width'),
      height: t('editor.hud.height'),
      stem: t('editor.hud.stem'),
    }),
    [t],
  )
  return useMemo<EditorLabels>(
    () => ({
      movePoints: (count) => t('editor.undo.movePoints', { count }),
      moveHandles: (count) => t('editor.undo.moveHandles', { count }),
      moveHandle: t('editor.undo.moveHandle'),
      reshapeSegment: t('editor.undo.reshapeSegment'),
      moveAnchor: t('editor.undo.moveAnchor'),
      moveComponent: t('editor.undo.moveComponent'),
      nudgePoints: (count) => t('editor.undo.nudgePoints', { count }),
      toggleSmooth: t('editor.undo.toggleSmooth'),
      deletePoints: t('editor.undo.deletePoints'),
      insertPoint: t('editor.undo.insertPoint'),
      convertSegment: t('editor.undo.convertSegment'),
      setMetrics: t('editor.undo.setMetrics'),
      addGuide: t('editor.undo.addGuide'),
      lockedTarget: t('editorMetadata.lockedTargetReason'),
      lockedGlyph: t('editorMetadata.glyphLockedReason'),
      metricHud: (handle, value) =>
        `${t(
          handle === 'width'
            ? 'glyphView.metric.advance'
            : handle === 'lsb'
              ? 'glyphView.metric.lsb'
              : 'glyphView.metric.rsb',
        )} ${value}`,
      snap: {
        node: t('editor.snap.node'),
        handle: t('editor.snap.handle'),
        guide: t('editor.snap.guide'),
        anchor: t('editor.snap.anchor'),
        grid: t('editor.snap.grid'),
        component: t('editor.snap.component'),
        background: t('editor.snap.background'),
        zone: t('editor.snap.zone'),
        advance: t('glyphView.metric.width'),
        baseline: t('glyphView.metric.baseline'),
        xHeight: t('glyphView.metric.xHeight'),
        capHeight: t('glyphView.metric.capHeight'),
        ascender: t('glyphView.metric.ascender'),
        descender: t('glyphView.metric.descender'),
      },
      hud: hud,
      tools: {
        addGuide: t('editor.undo.addGuide'),
        erase: t('editor.undo.erase'),
        drawPath: t('editor.undo.drawPath'),
        extendPath: t('editor.undo.extendPath'),
        draw: t('editor.undo.draw'),
        cut: t('editor.undo.cut'),
        rectangle: t('editor.undo.drawRectangle'),
        ellipse: t('editor.undo.drawEllipse'),
        rotate: t('editor.undo.rotate'),
        scale: t('editor.undo.scale'),
        distance: t('editor.hud.distance'),
        percent: t('editor.hud.percent'),
        hud,
      },
    }),
    [hud, t],
  )
}
