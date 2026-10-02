import {
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type Ref,
} from 'react'
import { useTranslation } from 'react-i18next'

import type { Contour } from '../../api'
import { call } from '../../api'
import type { Viewport } from '../../canvas/Viewport'
import { activeEditor } from '../../editor/activeEditor'
import type { EditorSource } from '../../editor/editorController'
import { editableContour, editorMetadata } from '../../editor/editorMetadata'
import { applyCommand } from '../../editor/editSession'
import { selectionOf } from '../../editor/selection'
import { useEditorStore } from '../../stores/editorStore'

import {
  auditOutline,
  OUTLINE_AUDIT_TESTS,
  outlineAuditFix,
  type OutlineAuditIssue,
} from './outlineAudit'
import { useOutlineAuditStore } from './outlineAuditStore'
import { OutlineAuditRepairError, repairAllOutline } from './outlineAuditRepair'
import styles from './outlineAudit.module.css'

/** The canvas calls this after pan, zoom or resize without a React rerender. */
export interface OutlineAuditHandle {
  refresh: () => void
}

/** How long a live gesture's geometry settles before it is re-audited. */
export const AUDIT_DEBOUNCE_MS = 120

/**
 * The longest the markers may lag the pointer during a continuous drag.
 *
 * A drag republishes its geometry every frame, so a plain debounce never fires
 * and the markers would sit at the gesture's first frame until the user paused.
 * This is the ceiling on that: the audit re-runs at least this often while the
 * outline keeps moving.
 */
export const AUDIT_MAX_WAIT_MS = 250

/**
 * The contours to audit, held still for `delay` milliseconds at a time.
 *
 * A drag republishes its optimistic geometry on every animation frame, and
 * checking a 300-point glyph that often would compete with the pointer for the
 * frame budget. Outside a gesture nothing is delayed, so a committed edit is
 * audited immediately.
 */
function useSettledContours(
  contours: readonly Contour[],
  delay: number,
): readonly Contour[] {
  const [settled, setSettled] = useState(contours)
  const [delaying, setDelaying] = useState(delay > 0)
  // When the currently shown geometry first went stale, so a drag that never
  // pauses still refreshes on the ceiling above.
  const staleSince = useRef(0)
  // React's documented way to follow a prop without an effect: notice during
  // render that a gesture has just started and adopt its first frame, so the
  // markers never show a position from the gesture before it.
  if (delaying !== delay > 0) {
    setDelaying(delay > 0)
    setSettled(contours)
  }
  useEffect(() => {
    // Outside a gesture there is nothing to hold back, and the clock is
    // cleared here rather than during render so the next gesture starts it
    // fresh without a render touching the ref.
    if (delay <= 0) {
      staleSince.current = 0
      return
    }
    const now = Date.now()
    if (staleSince.current === 0) staleSince.current = now
    const wait = Math.max(
      0,
      Math.min(delay, staleSince.current + AUDIT_MAX_WAIT_MS - now),
    )
    const timer = setTimeout(() => {
      staleSince.current = 0
      setSettled(contours)
    }, wait)
    return () => clearTimeout(timer)
  }, [contours, delay])
  return delay <= 0 ? contours : settled
}

/** Live checks and explicit repairs for the editable source layer. */
export function OutlineAudit({
  source,
  viewport,
  onReload,
  handleRef,
}: {
  source: EditorSource
  viewport: Viewport
  onReload: () => void
  handleRef?: Ref<OutlineAuditHandle>
}) {
  const { t } = useTranslation()
  const settings = useOutlineAuditStore((state) => state.settings)
  const setTest = useOutlineAuditStore((state) => state.setTest)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const editing = useEditorStore((state) => state.editing)
  // While a gesture runs the fetched outline is deliberately stale, so the
  // audit follows the geometry the canvas is actually drawing (design row 35:
  // the audit is always on, including mid-edit).
  const preview = useEditorStore((state) => state.preview)
  const markers = useRef(new Map<string, HTMLButtonElement>())
  const current = useRef(source)
  useLayoutEffect(() => {
    current.current = source
  }, [source])
  const live = useSettledContours(
    (editing && preview?.contours) || source.outline.contours,
    editing ? AUDIT_DEBOUNCE_MS : 0,
  )
  const report = useMemo(() => auditOutline(live, settings), [live, settings])
  const refresh = () => {
    for (const issue of report.issues) {
      const marker = markers.current.get(issue.id)
      if (!marker) continue
      const at = viewport.toScreen(issue)
      marker.style.left = `${at.x}px`
      marker.style.top = `${at.y}px`
    }
  }
  useImperativeHandle(handleRef, () => ({ refresh }))
  useLayoutEffect(refresh)

  // Two of the checks read differently depending on whether they can be
  // repaired, so the message names the actual repair instead of promising one.
  const detail = (issue: OutlineAuditIssue) => {
    const manual =
      issue.fix === undefined &&
      (issue.test === 'open' || issue.test === 'coincident')
    return t(`editor.audit.detail.${issue.test}${manual ? 'Manual' : ''}`, {
      x: issue.x,
      y: issue.y,
      count: issue.points.length,
    })
  }
  const lockedPaths = editorMetadata(source.editorMetadata).lockedPaths
  const repairable = (issue: OutlineAuditIssue) =>
    !source.glyphLocked &&
    issue.fix !== undefined &&
    issue.points.every((point) =>
      editableContour(
        source.outline.contours,
        point.contour,
        lockedPaths,
        null,
      ),
    )
  const fix = async (issues: readonly OutlineAuditIssue[], all = false) => {
    if (busy || editing || source.glyphLocked) return
    const captured = source
    setBusy(true)
    setError(null)
    try {
      await activeEditor()?.prepareTransition()
      if (current.current !== captured || useEditorStore.getState().editing)
        return
      if (all) {
        await repairAllOutline({
          target: {
            doc: source.doc,
            glyph: source.glyph,
            layer: source.outline.layer,
          },
          gid: source.gid,
          original: source.outline.contours,
          settings,
          lockedPaths,
          label: t('editor.audit.fixLabel'),
          isCurrent: () => {
            const live = current.current
            return (
              live.doc === captured.doc &&
              live.gid === captured.gid &&
              live.outline.layer === captured.outline.layer &&
              !live.glyphLocked &&
              !useEditorStore.getState().editing
            )
          },
        })
        return
      }
      const fresh = await call('glyphs.outline', {
        doc: source.doc,
        gid: source.gid,
        layer: source.outline.layer,
      })
      if (current.current !== captured) return
      if (
        JSON.stringify(fresh.contours) !==
        JSON.stringify(captured.outline.contours)
      ) {
        setError(t('editor.audit.stale'))
        return
      }
      const command = outlineAuditFix(
        { doc: source.doc, glyph: source.glyph, layer: fresh.layer },
        fresh.contours,
        issues.filter(repairable),
      )
      if (command)
        await applyCommand(source.doc, command, t('editor.audit.fixLabel'))
    } catch (cause) {
      setError(
        cause instanceof OutlineAuditRepairError
          ? t(`editor.audit.${cause.reason}`)
          : cause instanceof Error
            ? cause.message
            : String(cause),
      )
    } finally {
      setBusy(false)
      onReload()
    }
  }
  const hasFixes = report.issues.some(repairable)

  return (
    <div className={styles.overlay} aria-label={t('editor.audit.title')}>
      {report.issues.map((issue) => (
        <button
          key={issue.id}
          ref={(node) => {
            if (node) markers.current.set(issue.id, node)
            else markers.current.delete(issue.id)
          }}
          className={styles.marker}
          style={{
            left: viewport.toScreen(issue).x,
            top: viewport.toScreen(issue).y,
          }}
          type="button"
          title={detail(issue)}
          aria-label={t('editor.audit.marker', {
            test: t(`editor.audit.${issue.test}`),
            x: issue.x,
            y: issue.y,
          })}
          onClick={() => {
            const first = issue.points[0]
            if (!first) return
            const point =
              source.outline.contours[first.contour]?.points[first.point]
            activeEditor()?.setSelection(
              selectionOf([
                {
                  kind: point?.kind === 'OffCurve' ? 'handle' : 'node',
                  contour: first.contour,
                  point: first.point,
                },
              ]),
            )
          }}
        />
      ))}
      <details className={styles.panel}>
        <summary>
          {t('editor.audit.summary', { count: report.issues.length })}
        </summary>
        <fieldset className={styles.checks}>
          <legend>{t('editor.audit.checks')}</legend>
          {OUTLINE_AUDIT_TESTS.map((test) => (
            <label key={test}>
              <input
                type="checkbox"
                checked={settings[test]}
                onChange={(event) => setTest(test, event.target.checked)}
              />
              {t(`editor.audit.${test}`)}
            </label>
          ))}
        </fieldset>
        <button
          type="button"
          disabled={busy || editing || !hasFixes}
          onClick={() => {
            void fix(report.issues, true)
          }}
        >
          {t('editor.audit.fixAll')}
        </button>
        {error && <p role="alert">{error}</p>}
        {report.truncated && <p>{t('editor.audit.truncated')}</p>}
        {hasFixes && <p>{t('editor.audit.pass')}</p>}
        {report.issues.length === 0 && <p>{t('editor.audit.clean')}</p>}
        <ul className={styles.issues}>
          {report.issues.map((issue) => (
            <li key={issue.id}>
              <span title={detail(issue)}>{detail(issue)}</span>
              {repairable(issue) ? (
                <button
                  type="button"
                  disabled={busy || editing}
                  onClick={() => {
                    void fix([issue])
                  }}
                >
                  {t('editor.audit.fix')}
                </button>
              ) : (
                <small>{t('editor.audit.manual')}</small>
              )}
            </li>
          ))}
        </ul>
      </details>
    </div>
  )
}
