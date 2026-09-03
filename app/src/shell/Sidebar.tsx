import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { CountRow } from '../components/CountRow'
import { Panel } from '../components/Panel'
import { selectDocView, useDocumentStore } from '../stores/documentStore'
import { useSessionStore } from '../stores/sessionStore'

import { CountPanel } from './CountPanel'
import styles from './shell.module.css'

/**
 * Left sidebar of the Font view: glyph counts per category, script and Unicode
 * block. The rows become interactive filters with the font view.
 */
export function Sidebar() {
  const { t } = useTranslation()
  const activeDoc = useSessionStore((state) => state.activeDoc)
  const view = useDocumentStore(selectDocView(activeDoc))
  const loadSidebar = useDocumentStore((state) => state.loadSidebar)
  const counts = view.sidebar

  useEffect(() => {
    if (activeDoc !== null && counts === null) void loadSidebar(activeDoc)
  }, [activeDoc, counts, loadSidebar])

  return (
    <>
      <div className={styles.panelTitle}>{t('sidebar.title')}</div>
      <div className={styles.panelStack}>
        {activeDoc === null || counts === null ? (
          <p className={styles.hintBlock}>{t('sidebar.empty')}</p>
        ) : (
          <>
            <Panel title={t('sidebar.overview')}>
              <CountRow label={t('sidebar.all')} value={counts.total} />
              <CountRow
                label={t('sidebar.unencoded')}
                value={counts.unencoded}
              />
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
