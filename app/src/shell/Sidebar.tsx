import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { CountRow } from '../components/CountRow'
import { Panel } from '../components/Panel'
import { selectDocView, useDocumentStore } from '../stores/documentStore'
import { selectActiveTab, useUiStore } from '../stores/uiStore'

import { CountPanel } from './CountPanel'
import styles from './panels.module.css'

/**
 * Left pane. Its content follows the active tab: the Font tab gets the glyph
 * filter counts, other tab kinds get their own (still forthcoming) context.
 */
export function Sidebar() {
  const { t } = useTranslation()
  const tab = useUiStore(selectActiveTab)
  const isFontTab = tab?.kind === 'font'
  const doc = isFontTab ? tab.doc : null
  const view = useDocumentStore(selectDocView(doc))
  const loadSidebar = useDocumentStore((state) => state.loadSidebar)
  const counts = view.sidebar

  useEffect(() => {
    if (doc !== null && counts === null) void loadSidebar(doc)
  }, [doc, counts, loadSidebar])

  return (
    <>
      <div className={styles.panelTitle}>{t('sidebar.title')}</div>
      <div className={styles.panelStack}>
        {tab?.kind === 'glyph' ? (
          <p className={styles.hintBlock}>{t('sidebar.glyphNote')}</p>
        ) : tab?.kind === 'fontinfo' ? (
          <p className={styles.hintBlock}>{t('sidebar.fontInfoNote')}</p>
        ) : counts === null ? (
          <p className={styles.hintBlock}>{t('sidebar.empty')}</p>
        ) : (
          <>
            <Panel title={t('sidebar.overview')}>
              <CountRow label={t('sidebar.all')} value={counts.total} />
              <CountRow label={t('sidebar.unencoded')} value={counts.unencoded} />
            </Panel>
            <CountPanel
              title={t('sidebar.categories')}
              entries={counts.categories}
            />
            <CountPanel title={t('sidebar.scripts')} entries={counts.scripts} />
            <CountPanel title={t('sidebar.blocks')} entries={counts.blocks} />
            <p className={styles.hintBlock}>{t('sidebar.note')}</p>
          </>
        )}
      </div>
    </>
  )
}
