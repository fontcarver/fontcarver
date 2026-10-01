import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import type { DocSummary } from '../api'
import { documentLabel, useSessionStore } from '../stores/sessionStore'
import { useUiStore, type ClosePrompt } from '../stores/uiStore'

import styles from './dialog.module.css'
import { useModalChrome } from './useModalChrome'

export interface CloseDocumentPromptProps {
  prompt: ClosePrompt
  summary: DocSummary
}

/**
 * The modal body of the dirty-close prompt, mounted only while it is on screen.
 *
 * It is its own component so that `useModalChrome` runs on mount: the chrome
 * counts the prompt as a modal (keeping the global shortcuts, `Mod+W` among
 * them, off the document the prompt is asking about), captures the control to
 * return focus to before the `autoFocus` moves it, traps Tab inside and cancels
 * on Escape.
 */
export function CloseDocumentPrompt({
  prompt,
  summary,
}: CloseDocumentPromptProps) {
  const { t } = useTranslation()
  const setClosePrompt = useUiStore((state) => state.setClosePrompt)
  const saveAndClose = useSessionStore((state) => state.saveAndClose)
  const close = useSessionStore((state) => state.close)
  const dismiss = () => setClosePrompt(null)
  const chrome = useModalChrome(dismiss)

  return (
    <div
      className={styles.overlay}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) dismiss()
      }}
    >
      <div
        {...chrome}
        role="dialog"
        aria-modal="true"
        aria-label={t('dialog.closeDirty.title')}
        className={styles.dialog}
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
