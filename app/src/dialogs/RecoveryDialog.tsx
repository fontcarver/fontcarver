import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { RecoveryEntry } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import styles from './dialogs.module.css'

export interface RecoveryDialogProps {
  entries: readonly RecoveryEntry[]
}

/**
 * Recovery: what an interrupted session left behind. Recovering opens the
 * snapshot as a dirty document that keeps the original path — the banner in the
 * Font tab says so — and discarding asks once before deleting it for good.
 */
export function RecoveryDialog({ entries }: RecoveryDialogProps) {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const recover = useSessionStore((state) => state.recover)
  const discardRecovery = useSessionStore((state) => state.discardRecovery)
  const [remaining, setRemaining] = useState<readonly RecoveryEntry[]>(entries)
  const [confirming, setConfirming] = useState<string | null>(null)

  const close = () => setDialog(null)

  const drop = (entry: RecoveryEntry) => {
    const rest = remaining.filter(
      (candidate) => candidate.autosave_path !== entry.autosave_path,
    )
    setRemaining(rest)
    setConfirming(null)
    if (rest.length === 0) close()
  }

  return (
    <Dialog
      title={t('dialog.recovery.title')}
      size="lg"
      scrollBody
      onCancel={close}
      footer={<Button onClick={close}>{t('dialog.recovery.later')}</Button>}
    >
      <p className={styles.hint}>{t('dialog.recovery.intro')}</p>
      <div className={styles.list}>
        {remaining.map((entry) => (
          <div key={entry.autosave_path} className={styles.recoveryRow}>
            <span className={styles.recoveryText}>
              <span
                className={styles.recoveryPath}
                title={entry.doc_path ?? entry.autosave_path}
              >
                {entry.doc_path ?? t('dialog.recovery.untitled')}
              </span>
              <span className={styles.recoveryTime}>
                {t('dialog.recovery.savedAt', {
                  time: new Date(entry.saved_at_ms).toLocaleString(),
                })}
              </span>
            </span>
            {confirming === entry.autosave_path ? (
              <span className={styles.recoveryActions}>
                <span className={styles.hint}>
                  {t('dialog.recovery.confirmDiscard')}
                </span>
                <Button onClick={() => setConfirming(null)}>
                  {t('dialog.recovery.keep')}
                </Button>
                <Button
                  onClick={() => {
                    void discardRecovery(entry.autosave_path).then(
                      (discarded) => {
                        if (discarded) drop(entry)
                      },
                    )
                  }}
                >
                  {t('dialog.recovery.discard')}
                </Button>
              </span>
            ) : (
              <span className={styles.recoveryActions}>
                <Button onClick={() => setConfirming(entry.autosave_path)}>
                  {t('dialog.recovery.discard')}
                </Button>
                <Button
                  variant="primary"
                  onClick={() => {
                    void recover(entry.autosave_path).then((summary) => {
                      if (summary) drop(entry)
                    })
                  }}
                >
                  {t('dialog.recovery.recover')}
                </Button>
              </span>
            )}
          </div>
        ))}
      </div>
    </Dialog>
  )
}
