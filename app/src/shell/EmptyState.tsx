import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { useSessionStore } from '../stores/sessionStore'

import styles from './shell.module.css'

/** Shown in the content area while no document is open. */
export function EmptyState() {
  const { t } = useTranslation()
  const recent = useSessionStore((state) => state.recent)
  const openDialog = useSessionStore((state) => state.openDialog)
  const open = useSessionStore((state) => state.open)
  const createDocument = useSessionStore((state) => state.createDocument)

  return (
    <div className={styles.empty}>
      <div className={styles.emptyCard}>
        <h1 className={styles.emptyTitle}>{t('app.name')}</h1>
        <p className={styles.emptyTagline}>{t('empty.tagline')}</p>
        <div className={styles.emptyActions}>
          <Button variant="primary" large onClick={() => void openDialog()}>
            {t('empty.open')}
          </Button>
          <Button large onClick={() => void createDocument()}>
            {t('empty.new')}
          </Button>
        </div>
        <div className={styles.emptyRecent}>
          <h2 className={styles.emptyRecentTitle}>{t('empty.recent')}</h2>
          {recent.length === 0 ? (
            <p className={styles.hintBlock}>{t('empty.noRecent')}</p>
          ) : (
            <ul className={styles.recentList}>
              {recent.map((path) => (
                <li key={path}>
                  <button
                    type="button"
                    title={path}
                    className={styles.recentItem}
                    onClick={() => void open(path)}
                  >
                    {path}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
