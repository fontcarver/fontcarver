import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { reportError } from '../stores/uiStore'
import {
  useWorkspaceStore,
  type WorkspaceSnapshot,
} from '../stores/workspaceStore'

import styles from './WorkspaceDialog.module.css'

/** Document-specific capture and settled restore supplied by the shell. */
export interface WorkspaceDialogProps {
  documentKey: string
  capture: () => WorkspaceSnapshot | Promise<WorkspaceSnapshot>
  onApply: (snapshot: WorkspaceSnapshot) => boolean | Promise<boolean>
  onClose: () => void
}

/** Named workspace management; the latest layout is remembered automatically. */
export function WorkspaceDialog({
  documentKey,
  capture,
  onApply,
  onClose,
}: WorkspaceDialogProps) {
  const { t } = useTranslation()
  const workspace = useWorkspaceStore((state) => state.documents[documentKey])
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const presets = workspace?.presets ?? []
  const replacing = presets.some((preset) => preset.name === name.trim())

  const save = async () => {
    if (!name.trim() || busy) return
    setBusy(true)
    try {
      const snapshot = await capture()
      const saved = useWorkspaceStore
        .getState()
        .savePreset(documentKey, name, snapshot)
      setNotice(t(saved ? 'workspace.saved' : 'workspace.limit'))
      if (saved) setName('')
    } catch (error) {
      reportError(error)
    } finally {
      setBusy(false)
    }
  }

  const apply = async (presetName: string, snapshot: WorkspaceSnapshot) => {
    if (busy) return
    setBusy(true)
    try {
      if (await onApply(snapshot)) {
        useWorkspaceStore.getState().applyPreset(documentKey, presetName)
        setNotice(t('workspace.applied', { name: presetName }))
      }
    } catch (error) {
      reportError(error)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      title={t('workspace.title')}
      size="md"
      scrollBody
      onCancel={onClose}
      onConfirm={() => {
        void save()
      }}
      footer={<Button onClick={onClose}>{t('common.close')}</Button>}
    >
      <div className={styles.content}>
        <p className={styles.hint}>{t('workspace.help')}</p>
        <div className={styles.saveRow}>
          <label className={styles.name}>
            <span>{t('workspace.name')}</span>
            <input
              autoFocus
              className={styles.input}
              maxLength={80}
              value={name}
              disabled={busy}
              onChange={(event) => setName(event.target.value)}
            />
          </label>
          <Button
            variant="primary"
            disabled={!name.trim() || busy}
            onClick={() => {
              void save()
            }}
          >
            {t(replacing ? 'workspace.replace' : 'workspace.save')}
          </Button>
        </div>
        {presets.length ? (
          <ul className={styles.list} aria-label={t('workspace.savedList')}>
            {presets.map((preset) => (
              <li key={preset.name} className={styles.preset}>
                <span className={styles.presetName}>{preset.name}</span>
                <Button
                  disabled={busy}
                  aria-label={t('workspace.applyNamed', { name: preset.name })}
                  onClick={() => {
                    void apply(preset.name, preset.snapshot)
                  }}
                >
                  {t('workspace.apply')}
                </Button>
                <Button
                  disabled={busy}
                  aria-label={t('workspace.deleteNamed', { name: preset.name })}
                  onClick={() => {
                    useWorkspaceStore
                      .getState()
                      .deletePreset(documentKey, preset.name)
                    setNotice(t('workspace.deleted', { name: preset.name }))
                  }}
                >
                  {t('workspace.delete')}
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className={styles.hint}>{t('workspace.empty')}</p>
        )}
        <p role="status" className={styles.notice}>
          {notice}
        </p>
      </div>
    </Dialog>
  )
}
