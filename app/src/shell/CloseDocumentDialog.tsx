import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import {
  documentLabel,
  selectSummary,
  useSessionStore,
} from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import styles from './dialog.module.css'

/**
 * Save / Don't Save / Cancel prompt shown when a dirty document is closed.
 * Saving closes the document only once the kernel reports the write succeeded,
 * asking for a `.fcf` path first when the document has none.
 */
export function CloseDocumentDialog() {
  const { t } = useTranslation()
  const prompt = useUiStore((state) => state.closePrompt)
  const setClosePrompt = useUiStore((state) => state.setClosePrompt)
  const summary = useSessionStore(selectSummary(prompt?.doc ?? null))
  const saveAndClose = useSessionStore((state) => state.saveAndClose)
  const close = useSessionStore((state) => state.close)

  if (!prompt || !summary) return null

  const dismiss = () => setClosePrompt(null)

  return (
    <div
      className={styles.overlay}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) dismiss()
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t('dialog.closeDirty.title')}
        className={styles.dialog}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.stopPropagation()
            dismiss()
          }
        }}
      >
        <div className={styles.dialogHeader}>
          <h2 className={styles.dialogTitle}>{t('dialog.closeDirty.title')}</h2>
        </div>
        <div className={styles.dialogBody}>
          <p className={styles.dialogMessage}>
            {t('dialog.closeDirty.message', { name: documentLabel(summary) })}
          </p>
        </div>
        <div className={`${styles.dialogFooter} ${styles.dialogFooterSpread}`}>
          <Button
            onClick={() => {
              dismiss()
              void close(prompt.doc)
            }}
          >
            {t('dialog.closeDirty.discard')}
          </Button>
          <span>
            <Button onClick={dismiss}>{t('common.cancel')}</Button>{' '}
            <Button
              autoFocus
              variant="primary"
              onClick={() => {
                dismiss()
                void saveAndClose(prompt.doc)
              }}
            >
              {t('dialog.closeDirty.save')}
            </Button>
          </span>
        </div>
      </div>
    </div>
  )
}
