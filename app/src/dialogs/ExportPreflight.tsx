import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type DocId,
  type FindProblemsResult,
  type ProblemIssue,
} from '../api'
import { Button } from '../components/Button'
import { withSettledDocument } from '../editor/documentPersistence'
import { errorNotice, type ErrorNotice } from '../i18n/errors'
import { prepareTabTransition } from '../shell/tabActions'
import { useDocumentStore } from '../stores/documentStore'
import { useSessionStore } from '../stores/sessionStore'
import { activeDocId, selectActiveDoc, useUiStore } from '../stores/uiStore'

import styles from './FindProblemsDialog.module.css'
import formStyles from './dialogs.module.css'

interface ScanRequest {
  doc: DocId
}

interface ScanState {
  request: ScanRequest
  report?: FindProblemsResult
  error?: unknown
}

/** Explicit, read-only export checks with generation-safe glyph shortcuts. */
export function ExportPreflight({
  doc,
  busy,
  onNavigate,
}: {
  doc: DocId
  busy: boolean
  onNavigate: () => void
}) {
  const { t } = useTranslation()
  const activeDoc = useUiStore(selectActiveDoc)
  const summary = useSessionStore((state) =>
    state.docs.find((entry) => entry.doc === doc),
  )
  const generation = useDocumentStore(
    (state) => state.byDoc[doc]?.generation ?? 0,
  )
  const [request, setRequest] = useState<ScanRequest | null>(null)
  const [scan, setScan] = useState<ScanState | null>(null)
  const [navigationError, setNavigationError] = useState<{
    doc: DocId
    notice: ErrorNotice
  } | null>(null)
  const context = useRef(0)
  const isOpen = summary !== undefined
  const wrongDocument = activeDoc !== doc || !isOpen
  const matching = request?.doc === doc && scan?.request === request
  const report = matching ? scan.report : undefined
  const scanning = request?.doc === doc && !matching && !wrongDocument
  const stale = report !== undefined && generation > report.generation
  const failure =
    matching && scan.error !== undefined
      ? errorNotice(scan.error, (key, options) => t(key, options))
      : navigationError?.doc === doc
        ? navigationError.notice
        : null

  useEffect(() => {
    context.current += 1
    return () => {
      context.current += 1
    }
  }, [doc, activeDoc, isOpen, busy])

  useEffect(() => {
    if (!request || request.doc !== doc || matching || wrongDocument) return
    let cancelled = false
    const current = () =>
      !cancelled &&
      activeDocId() === doc &&
      useSessionStore.getState().docs.some((entry) => entry.doc === doc)
    useDocumentStore.getState().ensure(doc)
    void withSettledDocument(doc, () =>
      current()
        ? call('doc.find_problems', { doc, limit: 200 })
        : Promise.resolve(null),
    )
      .then((result) => {
        if (result?.doc === doc && current())
          setScan({ request, report: result })
      })
      .catch((error: unknown) => {
        if (current()) setScan({ request, error })
      })
    return () => {
      cancelled = true
    }
  }, [doc, request, matching, wrongDocument])

  const check = () => {
    if (busy || scanning || wrongDocument) return
    setNavigationError(null)
    setRequest({ doc })
  }

  const openIssue = async (issue: ProblemIssue) => {
    if (
      !report ||
      issue.glyph === null ||
      busy ||
      scanning ||
      stale ||
      wrongDocument
    )
      return
    const version = context.current
    const dialog = useUiStore.getState().dialog
    const transition = prepareTabTransition()
    try {
      if (!(await transition.ready) || !transition.isCurrent()) return
      const ui = useUiStore.getState()
      const documents = useDocumentStore.getState()
      const currentSummary = useSessionStore
        .getState()
        .docs.find((entry) => entry.doc === doc)
      // Report handles belong to this snapshot. Pending edits may have deleted
      // a glyph, switched documents, or closed this panel while we settled.
      if (
        context.current !== version ||
        ui.dialog !== dialog ||
        activeDocId() !== doc ||
        !currentSummary ||
        (documents.byDoc[doc]?.generation ?? 0) > report.generation
      )
        return
      if (
        issue.master &&
        currentSummary.masters.some((master) => master.id === issue.master)
      ) {
        documents.setMaster(doc, issue.master)
      }
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
      onNavigate()
    } catch (error) {
      if (
        context.current === version &&
        useUiStore.getState().dialog === dialog
      ) {
        setNavigationError({
          doc,
          notice: errorNotice(error, (key, options) => t(key, options)),
        })
      }
    }
  }

  const location = (issue: ProblemIssue) =>
    [
      issue.master
        ? (summary?.masters.find((master) => master.id === issue.master)
            ?.name ?? issue.master)
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
      defaultValue: issue.message,
    })

  return (
    <section
      className={formStyles.section}
      aria-label={t('exportPreflight.title')}
      aria-busy={scanning}
    >
      <div className={styles.toolbar}>
        <h3 className={formStyles.sectionTitle}>
          {t('exportPreflight.title')}
        </h3>
        <Button disabled={busy || scanning || wrongDocument} onClick={check}>
          {scanning
            ? t('dialog.problems.scanning')
            : t('exportPreflight.check')}
        </Button>
      </div>
      <p className={formStyles.hint}>{t('exportPreflight.help')}</p>
      {wrongDocument ? (
        <p role="status" className={styles.notice}>
          {t('exportPreflight.wrongDocument')}
        </p>
      ) : stale ? (
        <p role="status" className={styles.notice}>
          {t('exportPreflight.stale')}
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
      {report ? (
        <>
          <p role="status" className={styles.count}>
            {report.total === 0
              ? t('dialog.problems.empty')
              : t('exportPreflight.counts', {
                  errors: t('exportPreflight.errors', {
                    count: report.issues.filter(
                      (issue) => issue.severity === 'error',
                    ).length,
                  }),
                  warnings: t('exportPreflight.warnings', {
                    count: report.issues.filter(
                      (issue) => issue.severity === 'warning',
                    ).length,
                  }),
                  notes: t('exportPreflight.notes', {
                    count: report.issues.filter(
                      (issue) => issue.severity === 'info',
                    ).length,
                  }),
                })}
          </p>
          {report.has_errors ? (
            <p className={styles.notice}>{t('exportPreflight.hasErrors')}</p>
          ) : null}
          {report.truncated ? (
            <p className={styles.notice}>
              {t('exportPreflight.truncated', {
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
              <li className={styles.issue} key={`${issue.code}-${index}`}>
                <div className={styles.issueHeader}>
                  <span className={styles[issue.severity]}>
                    {t(`dialog.problems.severity.${issue.severity}`)}
                  </span>
                  <code className={styles.code}>{issue.code}</code>
                  {issue.glyph !== null ? (
                    <button
                      type="button"
                      className={styles.glyph}
                      disabled={busy || scanning || stale || wrongDocument}
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
    </section>
  )
}
