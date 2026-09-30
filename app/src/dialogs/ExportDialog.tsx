import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { withSettledDocument } from '../editor/documentPersistence'

import {
  call,
  type BitmapState,
  type DocSummary,
  type ExportFormat,
  type ExportOptions,
  type ExportReport,
} from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { errorNotice, type ErrorNotice } from '../i18n/errors'
import { useProgressStore } from '../stores/progressStore'
import { hasNativeFileDialog } from '../shell/nativeDialogs'
import { useUiStore } from '../stores/uiStore'

import styles from './dialogs.module.css'
import {
  BITMAP_FORMATS,
  ALL_MASTER_FORMATS,
  DIRECTORY_FORMATS,
  SFNT_FORMATS,
  chooseExportPath,
  formatsFor,
  suggestPath,
} from './exportFormats'
import { ExportResult } from './ExportResult'
import { ExportFormatTabs } from './ExportFormatTabs'
import { ExportVariationOptions } from './ExportVariationOptions'
import { ExportProfiles } from './ExportProfiles'
import { ExportPreflight } from './ExportPreflight'
import { ExportMatrixPanel } from './ExportMatrixPanel'
import {
  DEFAULT_EXPORT_PROCESSING,
  exportOptionsFor,
  processingFromProfile,
} from './exportOptionState'

import {
  ExportProcessingOptions,
  type ExportOptionField,
} from './ExportProcessingOptions'

export interface ExportDialogProps {
  summary: DocSummary
}

/**
 * Export: format tabs, a destination, the options `ExportOptions` understands,
 * a master picker and the report of the finished run. The busy line under the
 * tab bar reports progress; the dialog never blocks the shell with a spinner.
 */
export function ExportDialog({ summary }: ExportDialogProps) {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const formats = formatsFor(summary.masters.length)
  const [format, setFormat] = useState<ExportFormat>('Ttf')
  const [variable, setVariable] = useState(false)
  const [instance, setInstance] = useState<number>()
  const [masterSubset, setMasterSubset] = useState<string[]>()
  const canVary = SFNT_FORMATS.includes(format) && summary.axes.length > 0
  const canVariable = canVary && !['Cff', 'Type42'].includes(format)
  const effectiveVariable = canVariable && variable
  const [path, setPath] = useState(() => suggestPath(summary, 'Ttf'))
  const [master, setMaster] = useState<string | null>(
    summary.masters[0]?.id ?? null,
  )
  const [options, setOptions] = useState<Record<ExportOptionField, boolean>>({
    ...DEFAULT_EXPORT_PROCESSING,
  })
  const [glyph, setGlyph] = useState('')
  const [busy, setBusy] = useState(false)
  const [report, setReport] = useState<ExportReport | null>(null)
  const [failure, setFailure] = useState<ErrorNotice | null>(null)
  const [strikes, setStrikes] = useState<BitmapState['strikes']>([])
  const [bitmapKey, setBitmapKey] = useState('')
  const bitmapFormat = BITMAP_FORMATS.includes(format)
  useEffect(() => {
    if (!bitmapFormat) return
    let current = true
    void call('bitmap.state', { doc: summary.doc })
      .then((result) => {
        if (!current) return
        setStrikes(result.strikes)
        setBitmapKey(
          (key) =>
            key ||
            (result.strikes[0]
              ? `${result.strikes[0].ppem}:${result.strikes[0].depth}`
              : ''),
        )
      })
      .catch((error) => {
        if (current)
          setFailure(errorNotice(error, (key, options) => t(key, options)))
      })
    return () => {
      current = false
    }
  }, [bitmapFormat, summary.doc, t])
  const allMasters = ALL_MASTER_FORMATS.includes(format)

  const close = () => setDialog(null)

  // The dialog reports its own failures in place rather than as a toast, so it
  // renders what the shared reporter would have shown: the translated summary
  // for the kernel's error code, with the kernel's own wording under "Details".
  const noticeFor = (error: unknown): ErrorNotice =>
    errorNotice(error, (key, options) => t(key, options))

  const chooseFormat = (next: ExportFormat) => {
    setFormat(next)
    setReport(null)
    setFailure(null)
    // The suggested name follows the format unless the user typed their own.
    setPath((current) =>
      current === suggestPath(summary, format)
        ? suggestPath(summary, next)
        : current,
    )
  }

  const browse = () => {
    void chooseExportPath(path, t('dialog.export.title'), format)
      .then((chosen) => {
        if (chosen) setPath(chosen)
      })
      .catch((error: unknown) => setFailure(noticeFor(error)))
  }

  const exportOptions = exportOptionsFor({
    format,
    canVary,
    variable: effectiveVariable,
    instance,
    masterSubset,
    bitmapKey,
    master,
    glyph,
    options,
  })
  const applyProfile = (profile: ExportOptions) => {
    chooseFormat(profile.format)
    setVariable(profile.variable)
    setInstance(profile.instance)
    setMasterSubset(
      profile.master_subset?.filter((id) =>
        summary.masters.some((entry) => entry.id === id),
      ),
    )
    setMaster(
      summary.masters.some((entry) => entry.id === profile.master)
        ? profile.master
        : (summary.masters[0]?.id ?? null),
    )
    setOptions(processingFromProfile(profile))
    setBitmapKey(profile.bitmap_strike?.join(':') ?? '')
    setGlyph(profile.glyph ?? '')
  }
  const cannotRun =
    busy ||
    path.trim().length === 0 ||
    (format === 'Eps' && glyph.trim().length === 0)

  const run = () => {
    if (cannotRun) return
    setBusy(true)
    setFailure(null)
    setReport(null)
    // Show the busy line at once: the first kernel progress event only arrives
    // once compilation has started, which on a large font is not immediate.
    useProgressStore.getState().begin(summary.doc)
    void withSettledDocument(summary.doc, () =>
      call('session.export', {
        doc: summary.doc,
        path: path.trim(),
        options: exportOptions,
      }),
    )
      .then(setReport)
      .catch((error: unknown) => setFailure(noticeFor(error)))
      .finally(() => {
        useProgressStore.getState().clear(summary.doc)
        setBusy(false)
      })
  }

  return (
    <Dialog
      title={t('dialog.export.title')}
      size="md"
      scrollBody
      onCancel={close}
      onConfirm={run}
      footer={
        <>
          <Button onClick={close}>{t('common.close')}</Button>
          <Button variant="primary" disabled={cannotRun} onClick={run}>
            {busy
              ? t('dialog.export.running')
              : report
                ? t('dialog.export.again')
                : t('dialog.export.run')}
          </Button>
        </>
      }
    >
      {bitmapFormat && (
        <div>
          <p>{t('bitmap.exportHint')}</p>
          {['Bdf', 'Pcf', 'Psf', 'Psf1', 'Fnt'].includes(format) && (
            <label>
              {t('bitmap.exportStrike')}
              <select
                value={bitmapKey}
                onChange={(event) => setBitmapKey(event.target.value)}
              >
                {strikes.map((strike) => (
                  <option
                    key={`${strike.ppem}:${strike.depth}`}
                    value={`${strike.ppem}:${strike.depth}`}
                  >
                    {strike.ppem}px · {strike.depth}bpp
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
      )}
      <div className={styles.form}>
        <ExportProfiles
          options={exportOptions}
          busy={busy}
          onApply={applyProfile}
        />
        <ExportFormatTabs
          formats={formats}
          format={format}
          busy={busy}
          onChange={chooseFormat}
        />

        {format === 'Otf' ? (
          <p className={styles.hint}>
            {t(variable ? 'variation.cff2Hint' : 'dialog.export.cffHint')}
          </p>
        ) : null}
        {format === 'Pfa' || format === 'Pfb' ? (
          <p role="note" className={styles.formatWarning}>
            {t('dialog.export.type1Hint')}
          </p>
        ) : null}
        {allMasters ? (
          <p className={styles.hint}>{t('dialog.export.allMasters')}</p>
        ) : null}

        <div className={`${styles.row} ${styles.rowTop}`}>
          <span className={styles.rowLabel}>
            {t('dialog.export.destination')}
          </span>
          <div className={styles.rowStack}>
            <div className={styles.pathRow}>
              <input
                aria-label={t('dialog.export.destinationLabel')}
                className={styles.text}
                disabled={busy}
                value={path}
                onChange={(event) => setPath(event.target.value)}
              />
              {hasNativeFileDialog() ? (
                <Button disabled={busy} onClick={browse}>
                  {t('dialog.export.choose')}
                </Button>
              ) : null}
            </div>
            <p className={styles.hint}>
              {DIRECTORY_FORMATS.includes(format)
                ? t('dialog.export.directoryHint')
                : null}{' '}
              {hasNativeFileDialog() ? null : t('dialog.export.pathHint')}
            </p>
          </div>
        </div>

        {format === 'Eps' ? (
          <label className={styles.row}>
            <span className={styles.rowLabel}>
              {t('dialog.export.epsGlyph')}
            </span>
            <input
              className={styles.text}
              disabled={busy}
              value={glyph}
              onChange={(event) => setGlyph(event.target.value)}
            />
          </label>
        ) : null}
        {format === 'Ttc' ? (
          <p className={styles.hint}>{t('dialog.export.collectionHint')}</p>
        ) : null}

        {canVary && (
          <ExportVariationOptions
            doc={summary.doc}
            busy={busy}
            canVariable={canVariable}
            variable={effectiveVariable}
            onVariable={setVariable}
            instance={instance}
            onInstance={setInstance}
            subset={masterSubset}
            onSubset={setMasterSubset}
          />
        )}
        {!allMasters &&
        !(canVary && (effectiveVariable || instance !== undefined)) &&
        summary.masters.length > 0 ? (
          <label className={styles.row}>
            <span className={styles.rowLabel}>{t('dialog.export.master')}</span>
            <select
              className={styles.text}
              disabled={busy}
              value={master ?? ''}
              onChange={(event) => setMaster(event.target.value || null)}
            >
              {summary.masters.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
            </select>
          </label>
        ) : null}

        <ExportProcessingOptions
          format={format}
          canVary={canVary}
          variable={effectiveVariable}
          instance={instance}
          busy={busy}
          options={options}
          onChange={(field, value) =>
            setOptions((current) => ({ ...current, [field]: value }))
          }
        />

        <ExportPreflight doc={summary.doc} busy={busy} onNavigate={close} />
        <ExportMatrixPanel
          summary={summary}
          destination={path}
          busy={busy}
          onBusyChange={setBusy}
        />

        {failure ? (
          <div role="alert" className={styles.error}>
            <p className={styles.errorMessage}>{failure.message}</p>
            {failure.details === null ? null : (
              <details>
                <summary className={styles.errorSummary}>
                  {t('toast.details')}
                </summary>
                <p className={styles.errorDetail}>{failure.details}</p>
              </details>
            )}
          </div>
        ) : null}

        {report ? <ExportResult doc={summary.doc} report={report} /> : null}
      </div>
    </Dialog>
  )
}
