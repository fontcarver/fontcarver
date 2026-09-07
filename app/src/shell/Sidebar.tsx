import { useTranslation } from 'react-i18next'

import { EditSidebar } from '../views/glyph/EditSidebar'
import { SidebarFilters } from '../views/font/SidebarFilters'
import { selectActiveTab, useUiStore } from '../stores/uiStore'

import styles from './panels.module.css'

/**
 * Left pane. Its content follows the active tab: the Font tab gets the glyph
 * filter counts, an Edit tab the layers of its glyph over the Font view's
 * current glyph list (design spec §3).
 */
export function Sidebar() {
  const { t } = useTranslation()
  const tab = useUiStore(selectActiveTab)

  return (
    <>
      <div className={styles.panelTitle}>
        {tab?.kind === 'glyph' ? t('sidebar.glyphTitle') : t('sidebar.title')}
      </div>
      <div
        className={styles.panelStack}
        role="region"
        aria-label={t('a11y.sidebar')}
      >
        {tab?.kind === 'glyph' ? (
          <EditSidebar doc={tab.doc} />
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
