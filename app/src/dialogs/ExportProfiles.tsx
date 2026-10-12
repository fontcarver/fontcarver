import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId, ExportOptions, ExportProfile } from '../api'
import { Button } from '../components/Button'
import {
  ExportProfileError,
  type ExportProfileErrorCode,
  useExportProfileStore,
} from '../stores/exportProfileStore'
import styles from './dialogs.module.css'

/**
 * Named export settings stored in the project file.
 *
 * The same records drive the instance/profile matrix and
 * `fontcarver build --profile`, so saving one here is what a continuous
 * integration build reads back.
 */
export function ExportProfiles({
  doc,
  options,
  busy,
  onApply,
}: {
  doc: DocId
  options: ExportOptions
  busy: boolean
  onApply: (profile: ExportProfile) => void
}) {
  const { t } = useTranslation()
  const id = useId()
  const profiles = useExportProfileStore((state) => state.stored(doc).profiles)
  const storeDetail = useExportProfileStore((state) => state.detail)
  const [name, setName] = useState('')
  const [subfolder, setSubfolder] = useState('')
  const [fileName, setFileName] = useState('')
  const [selectedName, setSelectedName] = useState('')
  const [error, setError] = useState<ExportProfileErrorCode | null>(null)
  const [detail, setDetail] = useState<string | null>(null)
  const selected = profiles.find((profile) => profile.name === selectedName)
  useEffect(() => {
    void useExportProfileStore
      .getState()
      .load(doc)
      .catch(() => undefined)
  }, [doc])

  const perform = (action: () => Promise<unknown>) => {
    setError(null)
    setDetail(null)
    void action().catch((cause: unknown) => {
      if (cause instanceof ExportProfileError) {
        setError(cause.code)
        setDetail(cause.detail)
      } else {
        setError('kernelRejected')
        setDetail(cause instanceof Error ? cause.message : String(cause))
      }
    })
  }

  const draft = (): ExportProfile => ({
    name,
    subfolder,
    file_name: fileName,
    options,
  })

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
      <p className={styles.hint}>{t('exportProfiles.projectHint')}</p>
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
              perform(async () => {
                const saved = await useExportProfileStore
                  .getState()
                  .saveProfile(doc, draft())
                setSelectedName(saved.name)
                setName('')
              })
            }
          >
            {t('exportProfiles.save')}
          </Button>
        </div>
      </div>
      <div className={styles.row}>
        <label className={styles.rowLabel} htmlFor={`${id}-subfolder`}>
          {t('exportProfiles.subfolder')}
        </label>
        <input
          id={`${id}-subfolder`}
          className={styles.text}
          value={subfolder}
          disabled={busy}
          maxLength={200}
          onChange={(event) => setSubfolder(event.target.value)}
        />
      </div>
      <div className={styles.row}>
        <label className={styles.rowLabel} htmlFor={`${id}-file`}>
          {t('exportProfiles.fileName')}
        </label>
        <input
          id={`${id}-file`}
          className={styles.text}
          value={fileName}
          disabled={busy}
          maxLength={200}
          onChange={(event) => setFileName(event.target.value)}
        />
      </div>
      <div className={styles.row}>
        <label className={styles.rowLabel} htmlFor={`${id}-selected`}>
          {t('exportProfiles.selected')}
        </label>
        <select
          id={`${id}-selected`}
          className={styles.text}
          value={selected?.name ?? ''}
          disabled={busy || profiles.length === 0}
          onChange={(event) => {
            setSelectedName(event.target.value)
            setError(null)
            setDetail(null)
            const chosen = profiles.find(
              (profile) => profile.name === event.target.value,
            )
            if (chosen) {
              setSubfolder(chosen.subfolder)
              setFileName(chosen.file_name)
            }
          }}
        >
          <option value="">{t('exportProfiles.choose')}</option>
          {profiles.map((profile) => (
            <option key={profile.name} value={profile.name}>
              {profile.name}
            </option>
          ))}
        </select>
      </div>
      <div className={styles.choices}>
        <Button
          disabled={busy || !selected}
          onClick={() => {
            if (selected) onApply(structuredClone(selected))
          }}
        >
          {t('exportProfiles.apply')}
        </Button>
        <Button
          disabled={busy || !selected}
          onClick={() =>
            perform(async () => {
              if (!selected) return
              await useExportProfileStore
                .getState()
                .saveProfile(
                  doc,
                  { ...draft(), name: selected.name },
                  selected.name,
                )
            })
          }
        >
          {t('exportProfiles.update')}
        </Button>
        <Button
          disabled={busy || !selected}
          onClick={() =>
            perform(async () => {
              if (!selected) return
              await useExportProfileStore
                .getState()
                .removeProfile(doc, selected.name)
              setSelectedName('')
            })
          }
        >
          {t('exportProfiles.remove')}
        </Button>
      </div>
      {error && (
        <p role="alert" className={styles.error}>
          {t(`exportProfiles.errors.${error}`)}
          {detail === null ? null : ` ${detail}`}
        </p>
      )}
      {!error && storeDetail !== null && (
        <p role="alert" className={styles.error}>
          {storeDetail}
        </p>
      )}
    </section>
  )
}
