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

/** Builds the editor's label bundle for the current language. */
export function useEditorLabels(): EditorLabels {
  const { t } = useTranslation()
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
      snap: {
        node: t('editor.snap.node'),
        handle: t('editor.snap.handle'),
        guide: t('editor.snap.guide'),
        baseline: t('glyphView.metric.baseline'),
        xHeight: t('glyphView.metric.xHeight'),
        capHeight: t('glyphView.metric.capHeight'),
        ascender: t('glyphView.metric.ascender'),
        descender: t('glyphView.metric.descender'),
      },
      hud: {
        deltaX: t('editor.hud.deltaX'),
        deltaY: t('editor.hud.deltaY'),
        angle: t('editor.hud.angle'),
        length: t('editor.hud.length'),
        width: t('editor.hud.width'),
        height: t('editor.hud.height'),
        stem: t('editor.hud.stem'),
      },
    }),
    [t],
  )
}
