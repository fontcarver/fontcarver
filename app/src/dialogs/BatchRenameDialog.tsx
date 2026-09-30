import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type GlyphId } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { withSettledDocument } from '../editor/documentPersistence'
import { useSessionStore } from '../stores/sessionStore'
import { reportError, useUiStore } from '../stores/uiStore'

import {
  InvalidSearchError,
  loadRenameInventory,
  planBatchRename,
  replaceGlyphNames,
  type RenameInventory,
  type RenamePlan,
} from './batchRename'
import styles from './dialogs.module.css'
import renameStyles from './batchRename.module.css'

/** Selection captured when the batch rename dialog opens. */
export interface BatchRenameDialogProps {
  readonly doc: DocId
  readonly glyphs: readonly GlyphId[]
}

/** Find/replace with preview and collision checking across the full font. */
export function BatchRenameDialog({ doc, glyphs }: BatchRenameDialogProps) {
  const { t } = useTranslation()
  const [inventory, setInventory] = useState<readonly RenameInventory[] | null>(
    null,
  )
  const [find, setFind] = useState('')
  const [replacement, setReplacement] = useState('')
  const [regex, setRegex] = useState(false)
  const [ignoreCase, setIgnoreCase] = useState(false)
  const [busy, setBusy] = useState(false)
  const [stale, setStale] = useState(false)
  const [loadFailed, setLoadFailed] = useState(false)
  const [page, setPage] = useState(0)
  const [temporaryPrefix] = useState(
    () => `_fcRename${crypto.randomUUID().replaceAll('-', '')}_`,
  )

  useEffect(() => {
    let live = true
    void loadRenameInventory(doc)
      .then((result) => {
        if (live) setInventory(result)
      })
      .catch((error: unknown) => {
        if (live) {
          setLoadFailed(true)
          reportError(error)
        }
      })
    return () => {
      live = false
    }
  }, [doc])

  const selected = useMemo(() => new Set(glyphs), [glyphs])
  const names = useMemo(
    () =>
      (inventory ?? [])
        .filter(({ gid }) => selected.has(gid))
        .map(({ name }) => name),
    [inventory, selected],
  )
  // A pattern the user has not finished typing is a message under the field,
  // not an exception: the preview simply stays empty until it compiles.
  const { plan, searchError } = useMemo<{
    plan: RenamePlan
    searchError: string | null
  }>(() => {
    const allNames = (inventory ?? []).map(({ name }) => name)
    try {
      return {
        plan: planBatchRename(
          allNames,
          replaceGlyphNames(names, find, replacement, { regex, ignoreCase }),
          temporaryPrefix,
        ),
        searchError: null,
      }
    } catch (error) {
      if (!(error instanceof InvalidSearchError)) throw error
      return {
        plan: { renames: [], issues: [], command: null },
        searchError: error.message,
      }
    }
  }, [inventory, names, find, replacement, regex, ignoreCase, temporaryPrefix])
  const missing = inventory !== null && names.length !== selected.size
  const canApply =
    inventory !== null && !busy && !missing && plan.command !== null
  const close = () => {
    if (!busy) useUiStore.getState().setDialog(null)
  }
  const pages = Math.max(1, Math.ceil(plan.renames.length / 100))
  const currentPage = Math.min(page, pages - 1)

  const apply = () => {
    if (!canApply || !plan.command) return
    setBusy(true)
    setStale(false)
    void withSettledDocument(doc, async () => {
      // A save/draft flush may have changed the inventory. Require a fresh
      // preview before applying to names that no longer identify this selection.
      const latest = await loadRenameInventory(doc)
      if (JSON.stringify(latest) !== JSON.stringify(inventory)) {
        setInventory(latest)
        setStale(true)
        return false
      }
      await call('doc.apply', {
        doc,
        command: plan.command!,
        label: t('dialog.batchRename.undo'),
      })
      return true
    })
      .then(async (applied) => {
        if (!applied) return
        useUiStore.getState().setDialog(null)
        await useSessionStore.getState().refreshSummary(doc)
      })
      .catch(reportError)
      .finally(() => setBusy(false))
  }

  return (
    <Dialog
      title={t('dialog.batchRename.title')}
      size="md"
      onCancel={close}
      onConfirm={apply}
      footer={
        <>
          <Button disabled={busy} onClick={close}>
            {t('common.cancel')}
          </Button>
          <Button variant="primary" disabled={!canApply} onClick={apply}>
            {t('dialog.batchRename.apply', { count: plan.renames.length })}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <p className={styles.hint}>
          {t('dialog.batchRename.hint', { count: selected.size })}
        </p>
        <label className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.batchRename.find')}
          </span>
          <input
            className={styles.text}
            value={find}
            disabled={busy}
            aria-invalid={searchError === null ? undefined : true}
            onChange={(event) => {
              setFind(event.target.value)
              setPage(0)
            }}
          />
        </label>
        <div className={styles.row}>
          <span className={styles.rowLabel} />
          <label className={styles.checkRow}>
            <input
              type="checkbox"
              checked={regex}
              disabled={busy}
              onChange={(event) => {
                setRegex(event.currentTarget.checked)
                setPage(0)
              }}
            />
            <span>{t('dialog.batchRename.regex')}</span>
          </label>
          <label className={styles.checkRow}>
            <input
              type="checkbox"
              checked={ignoreCase}
              disabled={busy}
              onChange={(event) => {
                setIgnoreCase(event.currentTarget.checked)
                setPage(0)
              }}
            />
            <span>{t('dialog.batchRename.ignoreCase')}</span>
          </label>
        </div>
        <label className={styles.row}>
          <span className={styles.rowLabel}>
            {t('dialog.batchRename.replace')}
          </span>
          <input
            className={styles.text}
            value={replacement}
            disabled={busy}
            onChange={(event) => {
              setReplacement(event.target.value)
              setPage(0)
            }}
          />
        </label>
        {searchError !== null ? (
          <p role="alert" className={styles.error}>
            {t('dialog.batchRename.invalidRegex', { message: searchError })}
          </p>
        ) : null}
        {regex ? (
          <p className={styles.hint}>{t('dialog.batchRename.regexHint')}</p>
        ) : null}
        {inventory === null ? (
          <p role="status">
            {t(
              loadFailed
                ? 'dialog.batchRename.loadFailed'
                : 'dialog.batchRename.loading',
            )}
          </p>
        ) : null}
        {stale || missing ? (
          <p role="alert" className={styles.error}>
            {t('dialog.batchRename.stale')}
          </p>
        ) : null}
        {plan.issues.length > 0 ? (
          <ul role="alert" className={styles.error}>
            {plan.issues.slice(0, 10).map((issue, index) => (
              <li key={index}>
                {t(`dialog.batchRename.issue.${issue.kind}`, {
                  name: issue.name,
                })}
              </li>
            ))}
            {plan.issues.length > 10 ? (
              <li>
                {t('dialog.batchRename.moreIssues', {
                  count: plan.issues.length - 10,
                })}
              </li>
            ) : null}
          </ul>
        ) : null}
        <div className={renameStyles.preview} tabIndex={0}>
          <table
            className={renameStyles.table}
            aria-label={t('dialog.batchRename.preview')}
          >
            <thead>
              <tr>
                <th>{t('dialog.batchRename.before')}</th>
                <th>{t('dialog.batchRename.after')}</th>
              </tr>
            </thead>
            <tbody>
              {plan.renames
                .slice(currentPage * 100, (currentPage + 1) * 100)
                .map(({ from, to }) => (
                  <tr key={from}>
                    <td>{from}</td>
                    <td>{to || t('dialog.batchRename.emptyName')}</td>
                  </tr>
                ))}
            </tbody>
          </table>
          {plan.renames.length === 0 ? (
            <p className={styles.previewEmpty}>
              {t('dialog.batchRename.noChanges')}
            </p>
          ) : null}
        </div>
        {pages > 1 ? (
          <div className={renameStyles.pages}>
            <Button
              disabled={currentPage === 0}
              onClick={() => setPage(currentPage - 1)}
            >
              {t('dialog.batchRename.previous')}
            </Button>
            <span>
              {t('dialog.batchRename.page', { page: currentPage + 1, pages })}
            </span>
            <Button
              disabled={currentPage === pages - 1}
              onClick={() => setPage(currentPage + 1)}
            >
              {t('dialog.batchRename.next')}
            </Button>
          </div>
        ) : null}
      </div>
    </Dialog>
  )
}
