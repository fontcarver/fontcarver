import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocSummary,
  type ResolvedExportProfile,
  type VariationState,
} from '../api'
import { Button } from '../components/Button'
import { withSettledDocument } from '../editor/documentPersistence'
import { errorNotice } from '../i18n/errors'
import { useExportProfileStore } from '../stores/exportProfileStore'
import { useProgressStore } from '../stores/progressStore'
import { activeDocId } from '../stores/uiStore'
import { useSessionStore } from '../stores/sessionStore'
import {
  ExportMatrixError,
  planExportMatrix,
  runExportMatrix,
  type ExportMatrixCell,
  type ExportMatrixItem,
  type ExportMatrixResult,
} from './exportMatrix'
import { ExportResult } from './ExportResult'
import styles from './dialogs.module.css'

function parentDirectory(path: string): string {
  const separator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return separator < 0 ? '.' : path.slice(0, separator + 1)
}

/** Rows of the matrix: active named instances plus stored variable configurations. */
interface MatrixRow {
  kind: 'instance' | 'variable'
  name: string
}

/**
 * The instance/profile matrix.
 *
 * Every cell's options and file name come from `export.resolve_profile`, the
 * same kernel call `fontcarver build --profile` uses, so a matrix run and a CI
 * build of the same profile write the same files.
 */
export function ExportMatrixPanel({
  summary,
  destination,
  busy,
  onBusyChange,
}: {
  summary: DocSummary
  destination: string
  busy: boolean
  onBusyChange: (value: boolean) => void
}) {
  const { t } = useTranslation()
  const stored = useExportProfileStore((state) => state.stored(summary.doc))
  const [directory, setDirectory] = useState(() => parentDirectory(destination))
  const [profileNames, setProfileNames] = useState<string[]>([])
  const [rowKeys, setRowKeys] = useState<string[]>([])
  const [variation, setVariation] = useState<VariationState | null>(null)
  const [loading, setLoading] = useState(false)
  const [failure, setFailure] = useState('')
  const [items, setItems] = useState<ExportMatrixItem[]>([])
  const [planError, setPlanError] = useState('')
  const [results, setResults] = useState<ExportMatrixResult[]>([])
  const [total, setTotal] = useState(0)
  const [isRunning, setIsRunning] = useState(false)
  const running = useRef(false)
  const mounted = useRef(false)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])
  const current = () =>
    mounted.current &&
    activeDocId() === summary.doc &&
    useSessionStore.getState().docs.some((doc) => doc.doc === summary.doc)

  const instances = (variation?.instances ?? []).filter(
    (instance) => instance.active,
  )
  const rows: MatrixRow[] = [
    ...instances.map((instance) => ({
      kind: 'instance' as const,
      name: instance.name,
    })),
    ...stored.variableConfigs.map((config) => ({
      kind: 'variable' as const,
      name: config.name,
    })),
  ]
  const key = (row: MatrixRow) => `${row.kind}:${row.name}`
  const selectedProfiles = stored.profiles.filter((profile) =>
    profileNames.includes(profile.name),
  )
  const selectedRows = rows.filter((row) => rowKeys.includes(key(row)))

  const load = async () => {
    if (loading || busy) return
    setLoading(true)
    setFailure('')
    try {
      const state = await withSettledDocument(summary.doc, () =>
        call('variation.state', { doc: summary.doc }),
      )
      await useExportProfileStore.getState().load(summary.doc)
      if (current()) {
        setVariation(state)
        setRowKeys([])
        setItems([])
      }
    } catch (error) {
      if (current()) setFailure(errorNotice(error, t).message)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }

  /** Ask the kernel for every selected cell, then validate the whole plan. */
  const plan = async () => {
    setPlanError('')
    setItems([])
    if (selectedProfiles.length === 0 || selectedRows.length === 0) {
      setPlanError(t('exportMatrix.error.empty-rows'))
      return
    }
    try {
      const cells: ExportMatrixCell[] = []
      for (const profile of selectedProfiles) {
        for (const row of selectedRows) {
          const resolved: ResolvedExportProfile = await call(
            'export.resolve_profile',
            {
              doc: summary.doc,
              profile: profile.name,
              variable_config: row.kind === 'variable' ? row.name : undefined,
              instance: row.kind === 'instance' ? row.name : undefined,
            },
          )
          cells.push({
            profileName: profile.name,
            rowKind: row.kind,
            rowName: row.name,
            options: resolved.options,
            relativePath: resolved.relative_path,
          })
        }
      }
      if (!current()) return
      setItems(planExportMatrix(directory, cells))
    } catch (error) {
      if (!mounted.current) return
      setPlanError(
        error instanceof ExportMatrixError
          ? t(`exportMatrix.error.${error.code}`)
          : (errorNotice(error, t).details ?? errorNotice(error, t).message),
      )
    }
  }

  const run = async () => {
    if (running.current || busy || items.length === 0 || planError) return
    running.current = true
    setIsRunning(true)
    onBusyChange(true)
    setFailure('')
    setResults([])
    setTotal(items.length)
    useProgressStore.getState().begin(summary.doc)
    try {
      const snapshot = items
      await withSettledDocument(summary.doc, async () => {
        if (!current()) return
        const latest = await call('variation.state', { doc: summary.doc })
        if (!variation || latest.generation !== variation.generation)
          throw new Error(t('exportMatrix.changed'))
        await runExportMatrix(
          snapshot,
          (item) =>
            call('session.export', {
              doc: summary.doc,
              path: item.path,
              options: item.options,
            }),
          (result) => {
            if (mounted.current) setResults((previous) => [...previous, result])
          },
          current,
        )
      })
    } catch (error) {
      if (mounted.current) setFailure(errorNotice(error, t).message)
    } finally {
      useProgressStore.getState().clear(summary.doc)
      running.current = false
      if (mounted.current) {
        setIsRunning(false)
        onBusyChange(false)
      }
    }
  }
  return (
    <details
      className={styles.section}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.stopPropagation()
      }}
    >
      <summary>{t('exportMatrix.title')}</summary>
      <p className={styles.hint}>{t('exportMatrix.help')}</p>
      <fieldset
        disabled={busy || isRunning || loading}
        className={styles.section}
      >
        <label className={styles.row}>
          <span className={styles.rowLabel}>{t('exportMatrix.directory')}</span>
          <input
            className={styles.text}
            value={directory}
            onChange={(event) => {
              setDirectory(event.target.value)
              setItems([])
            }}
          />
        </label>
        <Button onClick={() => void load()}>
          {loading ? t('exportMatrix.loading') : t('exportMatrix.load')}
        </Button>
        {variation && rows.length === 0 ? (
          <p>{t('exportMatrix.noInstances')}</p>
        ) : null}
        {rows.length > 0 ? (
          <fieldset className={styles.section}>
            <legend>{t('exportMatrix.instances')}</legend>
            {rows.map((row) => (
              <label key={key(row)} className={styles.checkRow}>
                <input
                  type="checkbox"
                  checked={rowKeys.includes(key(row))}
                  onChange={(event) => {
                    setItems([])
                    setRowKeys((previous) =>
                      event.target.checked
                        ? [...previous, key(row)]
                        : previous.filter((value) => value !== key(row)),
                    )
                  }}
                />
                {row.kind === 'variable'
                  ? t('exportMatrix.variableRow', { name: row.name })
                  : row.name}
              </label>
            ))}
          </fieldset>
        ) : null}
        <fieldset className={styles.section}>
          <legend>{t('exportMatrix.profiles')}</legend>
          {stored.profiles.length === 0 ? (
            <p className={styles.hint}>{t('exportMatrix.noProfiles')}</p>
          ) : (
            stored.profiles.map((profile) => (
              <label key={profile.name} className={styles.checkRow}>
                <input
                  type="checkbox"
                  checked={profileNames.includes(profile.name)}
                  onChange={(event) => {
                    setItems([])
                    setProfileNames((previous) =>
                      event.target.checked
                        ? [...previous, profile.name]
                        : previous.filter((value) => value !== profile.name),
                    )
                  }}
                />
                {profile.name}
              </label>
            ))
          )}
        </fieldset>
        <Button onClick={() => void plan()}>{t('exportMatrix.plan')}</Button>
      </fieldset>
      {planError || failure ? (
        <p role="alert" className={styles.error}>
          {planError || failure}
        </p>
      ) : null}
      {items.length > 0 ? (
        <details>
          <summary>
            {t('exportMatrix.preview', { count: items.length })}
          </summary>
          <ul className={styles.list}>
            {items.map((item) => (
              <li key={item.id} className={styles.listRow}>
                {item.path}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      <Button
        disabled={
          busy || isRunning || loading || items.length === 0 || !!planError
        }
        onClick={() => void run()}
      >
        {isRunning ? t('exportMatrix.running') : t('exportMatrix.run')}
      </Button>
      {total > 0 ? (
        <p role="status">
          {t('exportMatrix.progress', { done: results.length, total })}
        </p>
      ) : null}
      {results.length > 0 ? (
        <ul aria-label={t('exportMatrix.results')} className={styles.section}>
          {results.map(({ item, report, error }) => (
            <li key={item.id}>
              <strong>
                {item.rowName} · {item.profileName}
              </strong>
              {report ? (
                <>
                  <span> · {t('exportMatrix.succeeded')}</span>
                  <ExportResult doc={summary.doc} report={report} />
                </>
              ) : (
                <>
                  <p className={styles.reportPath}>{item.path}</p>
                  <p className={styles.error}>
                    {t('exportMatrix.failed')}:{' '}
                    {errorNotice(error, t).details ??
                      errorNotice(error, t).message}
                  </p>
                </>
              )}
            </li>
          ))}
        </ul>
      ) : null}
    </details>
  )
}
