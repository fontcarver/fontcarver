import {
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type Ref,
} from 'react'
import { useTranslation } from 'react-i18next'

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
  DEFAULT_OUTLINE_AUDIT,
  OUTLINE_AUDIT_TESTS,
  outlineAuditFix,
  type OutlineAuditIssue,
} from './outlineAudit'
import styles from './outlineAudit.module.css'

/** The canvas calls this after pan, zoom or resize without a React rerender. */
export interface OutlineAuditHandle {
  refresh: () => void
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
  const [settings, setSettings] = useState(DEFAULT_OUTLINE_AUDIT)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const editing = useEditorStore((state) => state.editing)
  const markers = useRef(new Map<string, HTMLButtonElement>())
  const current = useRef(source)
  useLayoutEffect(() => {
    current.current = source
  }, [source])
  const report = useMemo(
    () => auditOutline(source.outline.contours, settings),
    [source.outline.contours, settings],
  )
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

  const detail = (issue: OutlineAuditIssue) =>
    t(`editor.audit.detail.${issue.test}`, {
      x: issue.x,
      y: issue.y,
      count: issue.points.length,
    })
  const lockedPaths = editorMetadata(source.editorMetadata).lockedPaths
  const repairable = (issue: OutlineAuditIssue) =>
    !source.glyphLocked &&
    issue.test === 'fractional' &&
    issue.points.every((point) =>
      editableContour(
        source.outline.contours,
        point.contour,
        lockedPaths,
        null,
      ),
    )
  const fix = async (issues: readonly OutlineAuditIssue[]) => {
    if (busy || editing || source.glyphLocked) return
    const captured = source
    setBusy(true)
    setError(null)
    try {
      await activeEditor()?.prepareTransition()
      if (current.current !== captured || useEditorStore.getState().editing)
        return
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
      setError(cause instanceof Error ? cause.message : String(cause))
    } finally {
      setBusy(false)
      onReload()
    }
  }
  const hasFixes = report.issues.some(repairable)

  return (
    <div className={styles.overlay} aria-label={t('editor.audit.title')}>
      {!editing &&
        report.issues.map((issue) => (
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
                onChange={(event) =>
                  setSettings((previous) => ({
                    ...previous,
                    [test]: event.target.checked,
                  }))
                }
              />
              {t(`editor.audit.${test}`)}
            </label>
          ))}
        </fieldset>
        <button
          type="button"
          disabled={busy || editing || !hasFixes}
          onClick={() => {
            void fix(report.issues)
          }}
        >
          {t('editor.audit.fixAll')}
        </button>
        {error && <p role="alert">{error}</p>}
        {report.truncated && <p>{t('editor.audit.truncated')}</p>}
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
