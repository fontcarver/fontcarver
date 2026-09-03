import { useTranslation } from 'react-i18next'

import { getTransport } from '../api'
import { selectDocView, useDocumentStore } from '../stores/documentStore'
import {
  documentLabel,
  selectSummary,
  useSessionStore,
} from '../stores/sessionStore'
import { selectActiveDoc, useUiStore } from '../stores/uiStore'

import styles from './StatusBar.module.css'

/** Bottom status bar: document, counts, dirty state and transport. */
export function StatusBar() {
  const { t } = useTranslation()
  const doc = useUiStore(selectActiveDoc)
  const summary = useSessionStore(selectSummary(doc))
  const view = useDocumentStore(selectDocView(doc))

  return (
    <div className={styles.statusbar}>
      {summary ? (
        <>
          <span className={styles.statusName}>{documentLabel(summary)}</span>
          <span className={styles.statusNumeric}>
            {t('status.glyphs', { count: summary.glyph_count })}
          </span>
          {view.selection.size > 0 ? (
            <span className={styles.statusNumeric}>
              {t('status.selected', { count: view.selection.size })}
            </span>
          ) : null}
          <span className={summary.dirty ? styles.statusDirty : undefined}>
            {summary.dirty ? t('status.dirty') : t('status.clean')}
          </span>
          {summary.warnings.length > 0 ? (
            <span>{t('status.warnings', { count: summary.warnings.length })}</span>
          ) : null}
          <span className={styles.statusSpacer} />
          <span className={styles.statusNumeric}>
            {t('status.cellSize', { size: view.cellSize })}
          </span>
        </>
      ) : (
        <>
          <span className={styles.statusName}>{t('status.noDocument')}</span>
          <span className={styles.statusSpacer} />
        </>
      )}
      <span>{t('status.transport', { kind: getTransport().kind })}</span>
    </div>
  )
}
