import { useCallback, useEffect, useMemo, useRef } from 'react'
import { call, type LayerOutline } from '../../api'
import { activeEditor } from '../../editor/activeEditor'
import { holdDocumentEdits } from '../../editor/editSession'
import { documentEditingSuspended } from '../../editor/documentEditParticipants'
import type { EditorController } from '../../editor/editorController'
import { reportError } from '../../shell/commands'
import { useEditorStore, type ActiveGlyph } from '../../stores/editorStore'
import { usePrefsStore } from '../../stores/prefsStore'
import { spacingLockedForDocument } from './kerningModes'
import { metricsGeometryPreview } from '../glyph/metricsHandles'
import {
  TextMetricsRunner,
  metricShortcut,
  type MetricShortcutEvent,
  type MetricSide,
} from './textMetrics'

interface Run {
  target: ActiveGlyph
  editor: EditorController
  runner: TextMetricsRunner
}

function sameTarget(a: ActiveGlyph | null, b: ActiveGlyph): boolean {
  return (
    a?.doc === b.doc &&
    a.gid === b.gid &&
    a.outline?.layer === b.outline?.layer &&
    a.master === b.master
  )
}

function outlineMetrics(outline: LayerOutline) {
  return { width: outline.width, lsb: outline.lsb, rsb: outline.rsb }
}

/** Captures each key before a subsequent command can close its edit session. */
export function useTextMetrics(
  enabled: boolean,
  label: string,
  side: MetricSide = 'lsb',
) {
  const currentRun = useRef<Run | null>(null)
  const runs = useRef(new Set<Run>())
  const alive = useRef(true)
  const finish = useCallback(() => {
    const closures = [...runs.current].map((run) =>
      holdDocumentEdits(run.target.doc, run.runner.end()),
    )
    return Promise.allSettled(closures).then(() => undefined)
  }, [])
  useEffect(() => {
    alive.current = true
    return () => {
      alive.current = false
      void finish()
    }
  }, [finish])

  const onMetricKey = useCallback(
    (event: MetricShortcutEvent): boolean => {
      const shortcut = metricShortcut(
        event,
        /Mac|iPhone|iPad/u.test(navigator.platform),
        { side, step: usePrefsStore.getState().prefs.nudge[0] },
      )
      if (!shortcut) return false
      const target = useEditorStore.getState().activeGlyph
      if (target && spacingLockedForDocument(target.doc)) return true
      if (!enabled) return false
      const editor = activeEditor()
      if (!editor || !target?.outline || !alive.current) return true
      if (documentEditingSuspended(target.doc)) return true
      const previous = currentRun.current
      if (
        previous?.runner.active &&
        previous.editor === editor &&
        sameTarget(target, previous.target)
      ) {
        previous.runner.press(shortcut.side, shortcut.delta)
        return true
      }

      // Preparation captures old work synchronously. Constructing the session now
      // captures that barrier before any later finish() can publish this run's
      // own closure; key presses therefore never hide in an async startup queue.
      // An idle timer can have started ending the prior run while controller
      // preparation is still pending. Publish that close even if the controller
      // reuses an already captured preparation promise.
      void finish()
      const after = editor.prepareExternalEdit()
      let outline = target.outline
      const record: Run = {
        target,
        editor,
        runner: new TextMetricsRunner({
          doc: target.doc,
          glyph: target.scene.name,
          layer: outline.layer,
          start: outlineMetrics(outline),
          hasOutline: outline.bbox !== null,
          label,
          after,
          refreshStart: async () => {
            // The previous tool may have changed geometry/metrics. Read the actual
            // layer after its closure, rather than reusing the old React snapshot.
            outline = await call('glyphs.outline', {
              doc: target.doc,
              gid: target.gid,
              layer: outline.layer,
            })
            return {
              metrics: outlineMetrics(outline),
              hasOutline: outline.bbox !== null,
            }
          },
          onPreview: (preview) => {
            if (
              !alive.current ||
              currentRun.current !== record ||
              activeEditor() !== editor ||
              !sameTarget(useEditorStore.getState().activeGlyph, target)
            )
              return
            editor.setExternalPreview(
              metricsGeometryPreview(outline, preview.lsbDelta),
              preview,
            )
          },
          onEnd: () => {
            runs.current.delete(record)
            if (currentRun.current !== record) return
            currentRun.current = null
            if (
              alive.current &&
              activeEditor() === editor &&
              sameTarget(useEditorStore.getState().activeGlyph, target)
            ) {
              editor.setExternalPreview(null, null)
              target.reload()
            }
          },
          onError: reportError,
        }),
      }
      runs.current.add(record)
      currentRun.current = record
      record.runner.press(shortcut.side, shortcut.delta)
      return true
    },
    [enabled, finish, label, side],
  )
  return useMemo(() => ({ finish, onMetricKey }), [finish, onMetricKey])
}
