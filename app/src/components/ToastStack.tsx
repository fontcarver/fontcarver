import { useTranslation } from 'react-i18next'

import { useUiStore } from '../stores/uiStore'

import styles from './controls.module.css'
import { Toast } from './Toast'

/** Notifications shown at once; older ones wait, per the design's §10. */
const MAX_VISIBLE = 3

/** The notification area anchored above the status bar. */
export function ToastStack() {
  const { t } = useTranslation()
  const toasts = useUiStore((state) => state.toasts)
  const dismissToast = useUiStore((state) => state.dismissToast)

  if (toasts.length === 0) return null
  const shown = toasts.slice(-MAX_VISIBLE)

  return (
    <div
      className={styles.toastStack}
      aria-label={t('a11y.notifications')}
      aria-live="polite"
    >
      {shown.map((toast) => (
        <Toast key={toast.id} toast={toast} onDismiss={dismissToast} />
      ))}
    </div>
  )
}
