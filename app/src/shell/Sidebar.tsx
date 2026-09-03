import { useTranslation } from 'react-i18next'

import { SidebarFilters } from '../views/font/SidebarFilters'
import { selectActiveTab, useUiStore } from '../stores/uiStore'

import styles from './panels.module.css'

/**
 * Left pane. Its content follows the active tab: the Font tab gets the glyph
 * filter counts, other tab kinds get their own (still forthcoming) context.
 */
export function Sidebar() {
  const { t } = useTranslation()
  const tab = useUiStore(selectActiveTab)

  return (
    <>
      <div className={styles.panelTitle}>{t('sidebar.title')}</div>
      <div className={styles.panelStack}>
        {tab?.kind === 'glyph' ? (
          <p className={styles.hintBlock}>{t('sidebar.glyphNote')}</p>
        ) : tab?.kind === 'fontinfo' ? (
          <p className={styles.hintBlock}>{t('sidebar.fontInfoNote')}</p>
        ) : tab?.kind === 'font' ? (
          <SidebarFilters doc={tab.doc} />
        ) : (
          <p className={styles.hintBlock}>{t('sidebar.empty')}</p>
        )}
      </div>
    </>
  )
}
