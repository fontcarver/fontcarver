import { useTranslation } from 'react-i18next'

import { useUiStore } from '../stores/uiStore'

import styles from './controls.module.css'
import { Toast } from './Toast'

/** The notification area anchored above the status bar. */
export function ToastStack() {
  const { t } = useTranslation()
  const toasts = useUiStore((state) => state.toasts)
  const dismissToast = useUiStore((state) => state.dismissToast)

  if (toasts.length === 0) return null

  return (
    <div
      className={styles.toastStack}
      aria-label={t('a11y.notifications')}
      aria-live="polite"
    >
      {toasts.map((toast) => (
        <Toast key={toast.id} toast={toast} onDismiss={dismissToast} />
      ))}
    </div>
  )
}
