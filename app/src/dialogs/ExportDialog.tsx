import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type DocSummary,
  type ExportFormat,
  type ExportOptions,
  type ExportReport,
} from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { warningMessage } from '../i18n/warnings'
import { useDocumentStore } from '../stores/documentStore'
import { useProgressStore } from '../stores/progressStore'
import { hasNativeFileDialog } from '../shell/nativeDialogs'
import { errorMessage } from '../stores/sessionStore'
import { tabId, useUiStore } from '../stores/uiStore'

import styles from './dialogs.module.css'
import {
  DIRECTORY_FORMATS,
  SFNT_FORMATS,
  chooseExportPath,
  formatsFor,
  suggestPath,
  warningGlyphNames,
} from './exportFormats'

/** The four checkboxes of the dialog and the `ExportOptions` field each writes. */
const OPTION_FIELDS = [
  'remove_overlap',
  'keep_tt_instructions',
  'preserve_untouched_tables',
  'production_names',
] as const

type OptionField = (typeof OPTION_FIELDS)[number]

/**
 * How many affected glyphs a warning shows as chips.
 *
 * `hinting-dropped` names every touched glyph, which on a heavily edited font is
 * thousands of them; the rest are summarised rather than rendered as thousands
 * of buttons in a modal.
 */
const MAX_CHIPS = 50

const OPTION_KEYS: Record<OptionField, string> = {
  remove_overlap: 'removeOverlap',
  keep_tt_instructions: 'keepTtInstructions',
  preserve_untouched_tables: 'preserveUntouchedTables',
  production_names: 'productionNames',
}

/**
 * Focuses one glyph in the document's Font tab.
 *
 * A warning chip is a shortcut into the grid, never a way to open a tab: when
 * the Font tab is closed the chip does nothing, as the dialog's own help says.
 */
async function selectGlyph(
  doc: DocSummary['doc'],
  name: string,
): Promise<void> {
  const ui = useUiStore.getState()
  if (!ui.tabs.some((tab) => tab.kind === 'font' && tab.doc === doc)) return
  const cells = await call('glyphs.search', { doc, query: name, limit: 8 })
  const cell = cells.find((candidate) => candidate.name === name)
  if (!cell) return
  const documents = useDocumentStore.getState()
  documents.setSelection(doc, [cell.gid])
  documents.setFocus(doc, cell.gid)
  ui.setActiveTab(tabId('font', doc))
}

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
  const [path, setPath] = useState(() => suggestPath(summary, 'Ttf'))
  const [master, setMaster] = useState<string | null>(
    summary.masters[0]?.id ?? null,
  )
  const [options, setOptions] = useState<Record<OptionField, boolean>>({
    remove_overlap: false,
    keep_tt_instructions: true,
    preserve_untouched_tables: true,
    production_names: false,
  })
  const [busy, setBusy] = useState(false)
  const [report, setReport] = useState<ExportReport | null>(null)
  const [failure, setFailure] = useState<string | null>(null)

  const close = () => setDialog(null)

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
      .catch((error: unknown) => setFailure(errorMessage(error)))
  }

  const run = () => {
    if (busy || path.trim().length === 0) return
    setBusy(true)
    setFailure(null)
    setReport(null)
    const exportOptions: ExportOptions = {
      format,
      master,
      remove_overlap: options.remove_overlap,
      keep_tt_instructions: options.keep_tt_instructions,
      preserve_untouched_tables: options.preserve_untouched_tables,
      production_names: options.production_names,
      autohint: false,
      table_subset: null,
    }
    // Show the busy line at once: the first kernel progress event only arrives
    // once compilation has started, which on a large font is not immediate.
    useProgressStore.getState().begin(summary.doc)
    void call('session.export', {
      doc: summary.doc,
      path: path.trim(),
      options: exportOptions,
    })
      .then(setReport)
      .catch((error: unknown) => setFailure(errorMessage(error)))
      .finally(() => {
        useProgressStore.getState().clear(summary.doc)
        setBusy(false)
      })
  }

  const showOptions = SFNT_FORMATS.includes(format)
  // Parsed once per report rather than twice per warning on every render.
  const warnings = useMemo(
    () =>
      (report?.warnings ?? []).map((warning) => ({
        warning,
        names: warningGlyphNames(warning),
      })),
    [report],
  )

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
          <Button
            variant="primary"
            disabled={busy || path.trim().length === 0}
            onClick={run}
          >
            {busy
              ? t('dialog.export.running')
              : report
                ? t('dialog.export.again')
                : t('dialog.export.run')}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <div
          role="tablist"
          aria-label={t('dialog.export.format')}
          className={styles.tabs}
        >
          {formats.map((candidate) => (
            <button
              key={candidate}
              type="button"
              role="tab"
              aria-selected={candidate === format}
              className={
                candidate === format
                  ? `${styles.tab} ${styles.tabActive}`
                  : styles.tab
              }
              onClick={() => chooseFormat(candidate)}
            >
              {t(`dialog.export.formatName.${candidate}`)}
            </button>
          ))}
        </div>

        <div className={`${styles.row} ${styles.rowTop}`}>
          <span className={styles.rowLabel}>
            {t('dialog.export.destination')}
          </span>
          <div className={styles.rowStack}>
            <div className={styles.pathRow}>
              <input
                aria-label={t('dialog.export.destinationLabel')}
                className={styles.text}
                value={path}
                onChange={(event) => setPath(event.target.value)}
              />
              {hasNativeFileDialog() ? (
                <Button onClick={browse}>{t('dialog.export.choose')}</Button>
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

        {summary.masters.length > 0 ? (
          <label className={styles.row}>
            <span className={styles.rowLabel}>{t('dialog.export.master')}</span>
            <select
              className={styles.text}
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

        {showOptions ? (
          <div className={styles.section}>
            <h3 className={styles.sectionTitle}>
              {t('dialog.export.options')}
            </h3>
            {OPTION_FIELDS.map((field) => (
              <div key={field} className={styles.checkRow}>
                <input
                  id={`export-${field}`}
                  type="checkbox"
                  checked={options[field]}
                  onChange={(event) =>
                    setOptions((current) => ({
                      ...current,
                      [field]: event.target.checked,
                    }))
                  }
                />
                <label
                  htmlFor={`export-${field}`}
                  className={styles.checkLabel}
                >
                  {t(`dialog.export.option.${OPTION_KEYS[field]}`)}
                </label>
                <span className={styles.checkHint}>
                  {t(`dialog.export.option.${OPTION_KEYS[field]}Hint`)}
                </span>
              </div>
            ))}
          </div>
        ) : null}

        {failure ? (
          <p role="alert" className={styles.error}>
            {failure}
          </p>
        ) : null}

        {report ? (
          <div className={styles.section}>
            <h3 className={styles.sectionTitle}>{t('dialog.export.result')}</h3>
            <p className={styles.reportPath} title={report.path}>
              {report.path}
            </p>
            <div className={styles.reportRow}>
              <span>
                {t('dialog.export.bytes', { count: report.bytes_written })}
              </span>
              <span>
                {t('dialog.export.touched', { count: report.touched_glyphs })}
              </span>
              <span>
                {t('dialog.export.timing', { count: report.timing_ms })}
              </span>
            </div>
            <h3 className={styles.sectionTitle}>
              {t('dialog.export.warnings')}
            </h3>
            {warnings.length === 0 ? (
              <p className={styles.hint}>{t('dialog.export.noWarnings')}</p>
            ) : (
              warnings.map(({ warning, names }) => (
                <div
                  key={warning.code + warning.message}
                  className={styles.warningItem}
                >
                  <span>
                    {warningMessage(warning, (key, translation) =>
                      translation ? t(key, translation) : t(key),
                    )}
                  </span>
                  {names.length > 0 ? (
                    <div className={styles.chips}>
                      {names.slice(0, MAX_CHIPS).map((name) => (
                        <button
                          key={name}
                          type="button"
                          title={t('dialog.export.selectGlyph', { name })}
                          className={styles.chip}
                          onClick={() => void selectGlyph(summary.doc, name)}
                        >
                          {name}
                        </button>
                      ))}
                      {names.length > MAX_CHIPS ? (
                        <span className={styles.hint}>
                          {t('dialog.export.moreGlyphs', {
                            count: names.length - MAX_CHIPS,
                          })}
                        </span>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              ))
            )}
          </div>
        ) : null}
      </div>
    </Dialog>
  )
}
