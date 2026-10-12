import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type ExportReport } from '../api'
import { warningMessage } from '../i18n/warnings'
import { reportError } from '../shell/commands'
import { prepareTabTransition } from '../shell/tabActions'
import { useDocumentStore } from '../stores/documentStore'
import { tabId, useUiStore } from '../stores/uiStore'

import styles from './dialogs.module.css'
import { warningGlyphNames } from './exportFormats'

const MAX_CHIPS = 50

/** Focuses an affected glyph after settling the editor that owns the old tab. */
async function selectGlyph(doc: DocId, name: string): Promise<void> {
  const ui = useUiStore.getState()
  if (!ui.tabs.some((tab) => tab.kind === 'font' && tab.doc === doc)) return
  const transition = prepareTabTransition()
  const cells = await call('glyphs.search', { doc, query: name, limit: 8 })
  const cell = cells.find((candidate) => candidate.name === name)
  if (!cell) return
  if (
    !(await transition.ready) ||
    !(await transition.settle()) ||
    !transition.isCurrent()
  )
    return
  const current = useUiStore.getState()
  if (!current.tabs.some((tab) => tab.kind === 'font' && tab.doc === doc))
    return
  const documents = useDocumentStore.getState()
  documents.setSelection(doc, [cell.gid])
  documents.setFocus(doc, cell.gid)
  current.setActiveTab(tabId('font', doc))
}

/** Export statistics and localized warnings with bounded glyph shortcuts. */
export function ExportResult({
  doc,
  report,
}: {
  doc: DocId
  report: ExportReport
}) {
  const { t } = useTranslation()
  const warnings = useMemo(
    () =>
      report.warnings.map((warning) => ({
        warning,
        names: warningGlyphNames(warning),
      })),
    [report],
  )
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>{t('dialog.export.result')}</h3>
      <p className={styles.reportPath} title={report.path}>
        {report.path}
      </p>
      <div className={styles.reportRow}>
        <span>{t('dialog.export.bytes', { count: report.bytes_written })}</span>
        <span>
          {t('dialog.export.touched', { count: report.touched_glyphs })}
        </span>
        <span>{t('dialog.export.timing', { count: report.timing_ms })}</span>
      </div>
      <h3 className={styles.sectionTitle}>{t('dialog.export.warnings')}</h3>
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
                    onClick={() =>
                      void selectGlyph(doc, name).catch(reportError)
                    }
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
  )
}
