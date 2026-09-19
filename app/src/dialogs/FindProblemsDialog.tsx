import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type DocSummary,
  type FindProblemsResult,
  type ProblemIssue,
} from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { withSettledDocument } from '../editor/documentPersistence'
import { errorNotice, type ErrorNotice } from '../i18n/errors'
import { prepareTabTransition } from '../shell/tabActions'
import { useDocumentStore } from '../stores/documentStore'
import { useSessionStore } from '../stores/sessionStore'
import { activeDocId, selectActiveDoc, useUiStore } from '../stores/uiStore'

import styles from './FindProblemsDialog.module.css'
import formStyles from './dialogs.module.css'

interface ScanState {
  doc: DocSummary['doc']
  request: number
  limit: number
  report?: FindProblemsResult
  error?: unknown
}

/** A bounded, read-only audit of the current document with explicit refresh. */
export function FindProblemsDialog({ summary }: { summary: DocSummary }) {
  const { t } = useTranslation()
  const { doc } = summary
  const activeDoc = useUiStore(selectActiveDoc)
  const generation = useDocumentStore(
    (state) => state.byDoc[doc]?.generation ?? 0,
  )
  const isOpen = useSessionStore((state) =>
    state.docs.some((entry) => entry.doc === doc),
  )
  const [request, setRequest] = useState(0)
  const [limit, setLimit] = useState(200)
  const [scan, setScan] = useState<ScanState | null>(null)
  const [navigationError, setNavigationError] = useState<ErrorNotice | null>(
    null,
  )
  const matching =
    scan?.doc === doc && scan.request === request && scan.limit === limit
  const report = matching ? scan.report : undefined
  const busy = !matching
  const wrongDocument = activeDoc !== doc || !isOpen
  const stale = report !== undefined && generation > report.generation
  const failure =
    matching && scan.error !== undefined
      ? errorNotice(scan.error, (key, options) => t(key, options))
      : navigationError

  useEffect(() => {
    // An unanswered request discarded during navigation can run again when
    // this document becomes active. Completed reports use explicit refresh.
    if (matching || activeDoc !== doc || !isOpen) return
    let cancelled = false
    const current = () =>
      !cancelled &&
      activeDocId() === doc &&
      useSessionStore.getState().docs.some((entry) => entry.doc === doc)
    if (current()) useDocumentStore.getState().ensure(doc)
    // Persist existing local edits first, then hold new edits until the scanner
    // has captured its snapshot. The scanner itself performs no repairs.
    void withSettledDocument(doc, () =>
      current()
        ? call('doc.find_problems', { doc, limit })
        : Promise.resolve(null),
    )
      .then((result) => {
        if (result?.doc === doc && current())
          setScan({ doc, request, limit, report: result })
      })
      .catch((error: unknown) => {
        if (current()) setScan({ doc, request, limit, error })
      })
    return () => {
      cancelled = true
    }
  }, [doc, limit, request, activeDoc, isOpen, matching])

  const close = () => useUiStore.getState().setDialog(null)
  const refresh = () => {
    setNavigationError(null)
    setRequest((current) => current + 1)
  }

  const openIssue = async (issue: ProblemIssue) => {
    if (!report || issue.glyph === null || wrongDocument || stale) return
    const dialog = useUiStore.getState().dialog
    const transition = prepareTabTransition()
    try {
      if (!(await transition.ready) || !transition.isCurrent()) return
      const ui = useUiStore.getState()
      const documents = useDocumentStore.getState()
      if (
        ui.dialog !== dialog ||
        activeDocId() !== doc ||
        !useSessionStore.getState().docs.some((entry) => entry.doc === doc) ||
        (documents.byDoc[doc]?.generation ?? 0) > report.generation
      )
        return
      if (
        issue.master &&
        summary.masters.some((master) => master.id === issue.master)
      )
        documents.setMaster(doc, issue.master)
      const existing = ui.tabs.find(
        (tab) =>
          tab.kind === 'glyph' && tab.doc === doc && tab.gid === issue.glyph,
      )
      if (existing) ui.setActiveTab(existing.id)
      else
        ui.openTab({
          kind: 'glyph',
          doc,
          gid: issue.glyph,
          label: issue.glyph_name ?? undefined,
        })
      close()
    } catch (error) {
      if (useUiStore.getState().dialog === dialog)
        setNavigationError(
          errorNotice(error, (key, options) => t(key, options)),
        )
    }
  }

  const location = (issue: ProblemIssue) =>
    [
      issue.master
        ? (summary.masters.find((master) => master.id === issue.master)?.name ??
          issue.master)
        : null,
      issue.layer ? t('dialog.problems.layer', { name: issue.layer }) : null,
      issue.color_layer !== null
        ? t('dialog.problems.colorLayer', { count: issue.color_layer + 1 })
        : null,
      issue.contour !== null
        ? t('dialog.problems.contour', { count: issue.contour + 1 })
        : null,
      issue.point !== null
        ? t('dialog.problems.point', { count: issue.point + 1 })
        : null,
    ]
      .filter(Boolean)
      .join(' · ')

  const issueMessage = (issue: ProblemIssue) =>
    t(`dialog.problems.code.${issue.code}`, {
      defaultValue: issue.code.startsWith('feature-')
        ? t('dialog.problems.featureDiagnostic')
        : issue.message,
    })

  return (
    <Dialog
      title={t('dialog.problems.title')}
      size="lg"
      scrollBody
      onCancel={close}
      footer={
        <>
          <Button onClick={close}>{t('common.close')}</Button>
          <Button
            variant="primary"
            disabled={busy || wrongDocument}
            onClick={refresh}
          >
            {busy && !wrongDocument
              ? t('dialog.problems.scanning')
              : t('dialog.problems.refresh')}
          </Button>
        </>
      }
    >
      <div className={formStyles.form}>
        <p className={styles.description}>{t('dialog.problems.help')}</p>
        <div className={styles.toolbar}>
          <span className={styles.fontName}>
            {summary.family_name} · {summary.style_name}
          </span>
          <label className={styles.limit}>
            {t('dialog.problems.limit')}
            <select
              className={formStyles.text}
              value={limit}
              disabled={busy || wrongDocument}
              onChange={(event) => {
                setNavigationError(null)
                setLimit(Number(event.target.value))
              }}
            >
              {[200, 500, 2000].map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
        </div>
        {wrongDocument ? (
          <p role="status" className={styles.notice}>
            {t('dialog.problems.wrongDocument')}
          </p>
        ) : stale ? (
          <p role="status" className={styles.notice}>
            {t('dialog.problems.stale')}
          </p>
        ) : null}
        {failure ? (
          <div role="alert" className={formStyles.error}>
            <p>{failure.message}</p>
            {failure.details ? (
              <details>
                <summary>{t('toast.details')}</summary>
                <p className={formStyles.errorDetail}>{failure.details}</p>
              </details>
            ) : null}
          </div>
        ) : null}
        {busy && !wrongDocument ? (
          <p role="status">{t('dialog.problems.scanning')}</p>
        ) : null}
        {report ? (
          <>
            <p role="status" className={styles.count}>
              {report.issues.length === 0
                ? t('dialog.problems.empty')
                : t('dialog.problems.count', { count: report.issues.length })}
            </p>
            {report.truncated ? (
              <p className={styles.notice}>
                {t('dialog.problems.truncated', {
                  shown: report.issues.length,
                  total: report.total,
                })}
              </p>
            ) : null}
            <ul
              className={styles.issues}
              aria-label={t('dialog.problems.results')}
            >
              {report.issues.map((issue, index) => (
                <li key={`${issue.code}-${index}`} className={styles.issue}>
                  <div className={styles.issueHeader}>
                    <span className={styles[issue.severity]}>
                      {t(`dialog.problems.severity.${issue.severity}`)}
                    </span>
                    <code className={styles.code}>{issue.code}</code>
                    {issue.glyph !== null ? (
                      <button
                        type="button"
                        className={styles.glyph}
                        disabled={busy || stale || wrongDocument}
                        aria-label={t('dialog.problems.openGlyph', {
                          name: issue.glyph_name ?? issue.glyph,
                        })}
                        onClick={() => void openIssue(issue)}
                      >
                        {issue.glyph_name ?? issue.glyph}
                      </button>
                    ) : (
                      <span>{t('dialog.problems.font')}</span>
                    )}
                  </div>
                  <p className={styles.message}>{issueMessage(issue)}</p>
                  {issueMessage(issue) !== issue.message ? (
                    <details className={styles.details}>
                      <summary>{t('toast.details')}</summary>
                      <p className={styles.message}>{issue.message}</p>
                    </details>
                  ) : null}
                  {location(issue) ? (
                    <p className={styles.location}>{location(issue)}</p>
                  ) : null}
                </li>
              ))}
            </ul>
          </>
        ) : null}
      </div>
    </Dialog>
  )
}
