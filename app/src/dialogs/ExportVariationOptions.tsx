import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId, type VariationState } from '../api'
import { errorNotice } from '../i18n/errors'
import styles from './dialogs.module.css'

/** Choose source masters or one static named instance from the saved design space. */
export function ExportVariationOptions({
  doc,
  busy,
  variable,
  onVariable,
  instance,
  onInstance,
  subset,
  onSubset,
}: {
  doc: DocId
  busy: boolean
  variable: boolean
  onVariable: (value: boolean) => void
  instance: number | undefined
  onInstance: (value: number | undefined) => void
  subset: string[] | undefined
  onSubset: (value: string[]) => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<VariationState | null>(null)
  const [error, setError] = useState('')
  useEffect(() => {
    let current = true
    void call('variation.state', { doc })
      .then((value) => {
        if (current) setState(value)
      })
      .catch((error: unknown) => {
        if (current) {
          const notice = errorNotice(error, t)
          setError(notice.details ?? notice.message)
        }
      })
    return () => {
      current = false
    }
  }, [doc, t])
  return (
    <fieldset disabled={busy} className={styles.section}>
      <label className={styles.row}>
        <input
          type="checkbox"
          checked={variable}
          onChange={(e) => onVariable(e.target.checked)}
        />
        {t('variation.exportVariable')}
      </label>
      {error && <p role="alert">{error}</p>}
      {state &&
        (variable ? (
          <fieldset>
            <legend>{t('variation.sourceMasters')}</legend>
            <p className={styles.hint}>{t('variation.subsetHint')}</p>
            {state.masters.map((master) => (
              <label key={master.id} className={styles.checkRow}>
                <input
                  type="checkbox"
                  checked={(subset ?? state.masters.map((m) => m.id)).includes(
                    master.id,
                  )}
                  onChange={(e) => {
                    const ids = subset ?? state.masters.map((m) => m.id)
                    onSubset(
                      e.target.checked
                        ? [...ids, master.id]
                        : ids.filter((id) => id !== master.id),
                    )
                  }}
                />
                {master.name}
              </label>
            ))}
          </fieldset>
        ) : (
          <label className={styles.row}>
            <span className={styles.rowLabel}>
              {t('variation.staticInstance')}
            </span>
            <select
              value={instance ?? ''}
              onChange={(e) =>
                onInstance(
                  e.target.value === '' ? undefined : Number(e.target.value),
                )
              }
            >
              <option value="">{t('variation.selectedMaster')}</option>
              {state.instances.map((item, index) => (
                <option key={index} value={index}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
        ))}
    </fieldset>
  )
}
