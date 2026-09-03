import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import type { Toast as ToastData } from '../stores/uiStore'

import styles from './controls.module.css'
import { IconButton } from './IconButton'

/** How long an informational toast stays on screen. Errors stay until dismissed. */
const AUTO_DISMISS_MS = 6_000

export interface ToastProps {
  toast: ToastData
  onDismiss: (id: string) => void
}

/** One non-blocking notification. */
export function Toast({ toast, onDismiss }: ToastProps) {
  const { t } = useTranslation()
  const { id, kind } = toast

  useEffect(() => {
    if (kind === 'error') return
    const timer = setTimeout(() => onDismiss(id), AUTO_DISMISS_MS)
    return () => clearTimeout(timer)
  }, [id, kind, onDismiss])

  const className =
    kind === 'error' ? `${styles.toast} ${styles.toastError}` : styles.toast

  return (
    <div className={className} role={kind === 'error' ? 'alert' : 'status'}>
      <span className={styles.toastMessage}>{toast.message}</span>
      <IconButton label={t('toast.dismiss')} onClick={() => onDismiss(id)}>
        ✕
      </IconButton>
    </div>
  )
}
