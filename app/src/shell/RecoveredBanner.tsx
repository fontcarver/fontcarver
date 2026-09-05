import { useTranslation } from 'react-i18next'

import type { DocSummary } from '../api'
import { IconButton } from '../components/IconButton'
import { useUiStore } from '../stores/uiStore'

import styles from './AppShell.module.css'

export interface RecoveredBannerProps {
  summary: DocSummary
}

/**
 * Warns that the open document came from a crash-recovery snapshot. It keeps
 * the original path, so the first save overwrites the file the snapshot belongs
 * to; the banner says so until the user dismisses it or saves.
 */
export function RecoveredBanner({ summary }: RecoveredBannerProps) {
  const { t } = useTranslation()
  const dismissed = useUiStore((state) => state.dismissedRecoveries)
  const dismiss = useUiStore((state) => state.dismissRecoveryBanner)
  const path = summary.recovered_from

  if (path === undefined || dismissed.includes(summary.doc)) return null

  return (
    <div role="status" className={styles.banner}>
      <span className={styles.bannerText} title={path}>
        {t('banner.recovered', { path })}
      </span>
      <IconButton
        label={t('banner.dismiss')}
        onClick={() => dismiss(summary.doc)}
      >
        ✕
      </IconButton>
    </div>
  )
}
