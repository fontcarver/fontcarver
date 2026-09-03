import { useTranslation } from 'react-i18next'

import { getTransport } from '../api'
import { selectDocView, useDocumentStore } from '../stores/documentStore'
import { useEditorStore } from '../stores/editorStore'
import {
  documentLabel,
  selectSummary,
  useSessionStore,
} from '../stores/sessionStore'
import { selectActiveDoc, selectActiveTab, useUiStore } from '../stores/uiStore'

import styles from './StatusBar.module.css'

/** Bottom status bar: document, counts, dirty state and transport. */
export function StatusBar() {
  const { t } = useTranslation()
  const doc = useUiStore(selectActiveDoc)
  const tab = useUiStore(selectActiveTab)
  const summary = useSessionStore(selectSummary(doc))
  const view = useDocumentStore(selectDocView(doc))
  const pointer = useEditorStore((state) => state.pointer)
  const zoom = useEditorStore((state) => state.zoom)
  const onGlyphTab = tab?.kind === 'glyph'

  return (
    <div
      className={styles.statusbar}
      role="contentinfo"
      aria-label={t('a11y.statusBar')}
    >
      {summary ? (
        <>
          <span className={styles.statusName}>{documentLabel(summary)}</span>
          <span className={styles.statusNumeric}>
            {t('status.glyphs', {
              count: view.matchCount ?? summary.glyph_count,
            })}
          </span>
          {view.matchCount !== null &&
          view.matchCount !== summary.glyph_count ? (
            <span className={styles.statusNumeric}>
              {t('status.ofTotal', { total: summary.glyph_count })}
            </span>
          ) : null}
          {view.selection.size > 0 ? (
            <span className={styles.statusNumeric}>
              {t('status.selected', { count: view.selection.size })}
            </span>
          ) : null}
          <span className={summary.dirty ? styles.statusDirty : undefined}>
            {summary.dirty ? t('status.dirty') : t('status.clean')}
          </span>
          {summary.warnings.length > 0 ? (
            <span>
              {t('status.warnings', { count: summary.warnings.length })}
            </span>
          ) : null}
          <span className={styles.statusSpacer} />
          {onGlyphTab ? (
            <>
              <span className={styles.statusNumeric}>
                {pointer
                  ? t('status.coordinates', { x: pointer.x, y: pointer.y })
                  : t('status.noCoordinates')}
              </span>
              <span className={styles.statusNumeric}>
                {t('status.zoom', {
                  size: Math.max(
                    1,
                    Math.round(zoom * (summary.units_per_em || 1000)),
                  ),
                })}
              </span>
            </>
          ) : (
            <span className={styles.statusNumeric}>
              {t('status.cellSize', { size: view.cellSize })}
            </span>
          )}
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
