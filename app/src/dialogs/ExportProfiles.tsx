import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { ExportOptions } from '../api'
import { Button } from '../components/Button'
import {
  ExportProfileError,
  type ExportProfileErrorCode,
  useExportProfileStore,
} from '../stores/exportProfileStore'
import styles from './dialogs.module.css'

/** Save and reuse named export settings independently of document destinations. */
export function ExportProfiles({
  options,
  busy,
  onApply,
}: {
  options: ExportOptions
  busy: boolean
  onApply: (options: ExportOptions) => void
}) {
  const { t } = useTranslation()
  const id = useId()
  const profiles = useExportProfileStore((state) => state.profiles)
  const storageError = useExportProfileStore((state) => state.error)
  const [name, setName] = useState('')
  const [selectedId, setSelectedId] = useState('')
  const [error, setError] = useState<ExportProfileErrorCode | null>(null)
  const selected = profiles.find((profile) => profile.id === selectedId)
  useEffect(() => {
    useExportProfileStore.getState().reload()
  }, [])

  const perform = (action: () => void) => {
    try {
      action()
      setError(null)
    } catch (cause) {
      setError(
        cause instanceof ExportProfileError ? cause.code : 'storageUnavailable',
      )
    }
  }

  return (
    <section
      className={styles.section}
      aria-labelledby={`${id}-heading`}
      onKeyDown={(event) => {
        // The containing dialog uses Enter to confirm a single export.
        if (event.key === 'Enter') event.stopPropagation()
      }}
    >
      <h3 id={`${id}-heading`} className={styles.sectionTitle}>
        {t('exportProfiles.title')}
      </h3>
      <p className={styles.hint}>{t('exportProfiles.hint')}</p>
      <div className={styles.row}>
        <label className={styles.rowLabel} htmlFor={`${id}-name`}>
          {t('exportProfiles.name')}
        </label>
        <div className={styles.pathRow}>
          <input
            id={`${id}-name`}
            className={styles.text}
            value={name}
            disabled={busy}
            maxLength={80}
            onChange={(event) => setName(event.target.value)}
          />
          <Button
            disabled={busy || !name.trim()}
            onClick={() =>
              perform(() => {
                const saved = useExportProfileStore
                  .getState()
                  .save(name, options)
                setSelectedId(saved.id)
                setName('')
              })
            }
          >
            {t('exportProfiles.save')}
          </Button>
        </div>
      </div>
      <div className={styles.row}>
        <label className={styles.rowLabel} htmlFor={`${id}-selected`}>
          {t('exportProfiles.selected')}
        </label>
        <select
          id={`${id}-selected`}
          className={styles.text}
          value={selected?.id ?? ''}
          disabled={busy || profiles.length === 0}
          onChange={(event) => {
            setSelectedId(event.target.value)
            setError(null)
          }}
        >
          <option value="">{t('exportProfiles.choose')}</option>
          {profiles.map((profile) => (
            <option key={profile.id} value={profile.id}>
              {profile.name}
            </option>
          ))}
        </select>
      </div>
      <div className={styles.choices}>
        <Button
          disabled={busy || !selected}
          onClick={() => {
            if (selected) onApply(structuredClone(selected.options))
          }}
        >
          {t('exportProfiles.apply')}
        </Button>
        <Button
          disabled={busy || !selected}
          onClick={() =>
            perform(() => {
              if (!selected) return
              useExportProfileStore.getState().remove(selected.id)
              setSelectedId('')
            })
          }
        >
          {t('exportProfiles.remove')}
        </Button>
      </div>
      {(error ?? storageError) && (
        <p role="alert" className={styles.error}>
          {t(`exportProfiles.errors.${error ?? storageError}`)}
        </p>
      )}
    </section>
  )
}
