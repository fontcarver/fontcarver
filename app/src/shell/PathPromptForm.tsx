import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { useSessionStore } from '../stores/sessionStore'
import { useUiStore, type PathPrompt } from '../stores/uiStore'

import styles from './dialog.module.css'

export interface PathPromptFormProps {
  prompt: PathPrompt
}

/** The modal body of the path prompt; remounted for each prompt. */
export function PathPromptForm({ prompt }: PathPromptFormProps) {
  const { t } = useTranslation()
  const setPathPrompt = useUiStore((state) => state.setPathPrompt)
  const recent = useSessionStore((state) => state.recent)
  const open = useSessionStore((state) => state.open)
  const save = useSessionStore((state) => state.save)
  const closeDocument = useSessionStore((state) => state.requestClose)
  const [path, setPath] = useState('')

  const isOpen = prompt.mode === 'open'
  const namespace = isOpen ? 'dialog.open' : 'dialog.saveAs'
  const close = () => setPathPrompt(null)

  const submit = (value: string) => {
    const trimmed = value.trim()
    if (!trimmed) return
    close()
    if (isOpen) {
      void open(trimmed)
      return
    }
    const doc = prompt.doc
    if (doc === undefined) return
    void save(doc, trimmed).then((saved) => {
      // The dirty-close flow only closes once the save actually succeeded.
      if (saved && prompt.closeAfterSave) return closeDocument(doc)
      return undefined
    })
  }

  return (
    <div
      className={styles.overlay}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) close()
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t(`${namespace}.title`)}
        className={styles.dialog}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.stopPropagation()
            close()
          }
        }}
      >
        <div className={styles.dialogHeader}>
          <h2 className={styles.dialogTitle}>{t(`${namespace}.title`)}</h2>
        </div>
        <div className={styles.dialogBody}>
          <input
            autoFocus
            className={styles.textInput}
            aria-label={t(`${namespace}.label`)}
            placeholder={t(`${namespace}.placeholder`)}
            value={path}
            onChange={(event) => setPath(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') submit(path)
            }}
          />
          <p className={styles.dialogHint}>{t(`${namespace}.hint`)}</p>
          {isOpen && recent.length > 0 ? (
            <ul className={styles.dialogRecent}>
              {recent.map((entry) => (
                <li key={entry}>
                  <button
                    type="button"
                    title={entry}
                    className={styles.recentItem}
                    onClick={() => submit(entry)}
                  >
                    {entry}
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
        <div className={styles.dialogFooter}>
          <Button onClick={close}>{t('common.cancel')}</Button>
          <Button
            variant="primary"
            disabled={path.trim().length === 0}
            onClick={() => submit(path)}
          >
            {t(`${namespace}.confirm`)}
          </Button>
        </div>
      </div>
    </div>
  )
}
