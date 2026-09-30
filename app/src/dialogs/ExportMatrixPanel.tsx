import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocSummary, type VariationState } from '../api'
import { Button } from '../components/Button'
import { withSettledDocument } from '../editor/documentPersistence'
import { errorNotice } from '../i18n/errors'
import { useExportProfileStore } from '../stores/exportProfileStore'
import { useProgressStore } from '../stores/progressStore'
import { activeDocId } from '../stores/uiStore'
import { useSessionStore } from '../stores/sessionStore'
import { SFNT_FORMATS } from './exportFormats'
import {
  ExportMatrixError,
  planExportMatrix,
  runExportMatrix,
  type ExportMatrixResult,
} from './exportMatrix'
import { ExportResult } from './ExportResult'
import styles from './dialogs.module.css'

function parentDirectory(path: string): string {
  const separator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return separator < 0 ? '.' : path.slice(0, separator + 1)
}

/** A bounded matrix of saved processing profiles and active named instances. */
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
  const profiles = useExportProfileStore((state) => state.profiles)
  const [directory, setDirectory] = useState(() => parentDirectory(destination))
  const [profileIds, setProfileIds] = useState<string[]>([])
  const [instanceIds, setInstanceIds] = useState<number[]>([])
  const [variation, setVariation] = useState<VariationState | null>(null)
  const [loading, setLoading] = useState(false)
  const [failure, setFailure] = useState('')
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
  const eligible = profiles.filter((profile) =>
    SFNT_FORMATS.includes(profile.options.format),
  )
  const instances = (variation?.instances ?? [])
    .map((instance, index) => ({ ...instance, index }))
    .filter((instance) => instance.active)
  const selectedProfiles = eligible.filter((profile) =>
    profileIds.includes(profile.id),
  )
  const selectedInstances = instances.filter((instance) =>
    instanceIds.includes(instance.index),
  )
  let planningError = ''
  let items: ReturnType<typeof planExportMatrix> = []
  if (selectedProfiles.length > 0 && selectedInstances.length > 0) {
    try {
      items = planExportMatrix(directory, selectedProfiles, selectedInstances)
    } catch (error) {
      planningError =
        error instanceof ExportMatrixError
          ? t(`exportMatrix.error.${error.code}`)
          : errorNotice(error, t).message
    }
  }

  const load = async () => {
    if (loading || busy) return
    setLoading(true)
    setFailure('')
    try {
      const state = await withSettledDocument(summary.doc, () =>
        call('variation.state', { doc: summary.doc }),
      )
      if (current()) {
        setVariation(state)
        setInstanceIds([])
      }
    } catch (error) {
      if (current()) setFailure(errorNotice(error, t).message)
    } finally {
      if (mounted.current) setLoading(false)
    }
  }
  const run = async () => {
    if (running.current || busy || items.length === 0 || planningError) return
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
            onChange={(event) => setDirectory(event.target.value)}
          />
        </label>
        <Button onClick={() => void load()}>
          {loading ? t('exportMatrix.loading') : t('exportMatrix.load')}
        </Button>
        {variation && instances.length === 0 ? (
          <p>{t('exportMatrix.noInstances')}</p>
        ) : null}
        {instances.length > 0 ? (
          <fieldset className={styles.section}>
            <legend>{t('exportMatrix.instances')}</legend>
            {instances.map((instance) => (
              <label key={instance.index} className={styles.checkRow}>
                <input
                  type="checkbox"
                  checked={instanceIds.includes(instance.index)}
                  onChange={(event) =>
                    setInstanceIds((previous) =>
                      event.target.checked
                        ? [...previous, instance.index]
                        : previous.filter((id) => id !== instance.index),
                    )
                  }
                />
                {instance.name}
              </label>
            ))}
          </fieldset>
        ) : null}
        <fieldset className={styles.section}>
          <legend>{t('exportMatrix.profiles')}</legend>
          {eligible.length === 0 ? (
            <p className={styles.hint}>{t('exportMatrix.noProfiles')}</p>
          ) : (
            eligible.map((profile) => (
              <label key={profile.id} className={styles.checkRow}>
                <input
                  type="checkbox"
                  checked={profileIds.includes(profile.id)}
                  onChange={(event) =>
                    setProfileIds((previous) =>
                      event.target.checked
                        ? [...previous, profile.id]
                        : previous.filter((id) => id !== profile.id),
                    )
                  }
                />
                {profile.name}
              </label>
            ))
          )}
        </fieldset>
      </fieldset>
      {planningError || failure ? (
        <p role="alert" className={styles.error}>
          {planningError || failure}
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
          busy || isRunning || loading || items.length === 0 || !!planningError
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
                {item.instanceName} · {item.profileName}
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
