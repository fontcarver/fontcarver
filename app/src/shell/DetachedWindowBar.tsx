import { useTranslation } from 'react-i18next'
import { Button } from '../components/Button'
import { reportError } from '../stores/uiStore'
import type { WindowTarget } from '../transport/windows'
import { rejoinDetachedWindow } from './windowActions'
import styles from './TabBar.module.css'

/** Lets a detached view return to the window that owns its document. */
export function DetachedWindowBar({ target }: { target: WindowTarget }) {
  const { t } = useTranslation()
  return (
    <div className={styles.tabbar}>
      <Button
        onClick={() => void rejoinDetachedWindow(target).catch(reportError)}
      >
        {t('shellWindow.rejoin')}
      </Button>
    </div>
  )
}
