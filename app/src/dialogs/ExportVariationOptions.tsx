import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId, type VariationState } from '../api'
import { Button } from '../components/Button'
import { errorNotice } from '../i18n/errors'
import {
  ExportProfileError,
  useExportProfileStore,
} from '../stores/exportProfileStore'
import styles from './dialogs.module.css'

/** Axis tag as the API reports it: four bytes, not a string. */
function axisTag(tag: number[] | string): string {
  return typeof tag === 'string' ? tag : String.fromCharCode(...tag)
}

export interface VariableSubset {
  masters: string[] | undefined
  axes: string[] | undefined
  instances: string[] | undefined
}

/**
 * Source masters, axes and named instances for a variable export, plus the
 * named variable-font configurations the project stores.
 */
export function ExportVariationOptions({
  doc,
  busy,
  canVariable = true,
  variable,
  onVariable,
  instance,
  onInstance,
  subset,
  onSubset,
}: {
  doc: DocId
  busy: boolean
  canVariable?: boolean
  variable: boolean
  onVariable: (value: boolean) => void
  instance: number | undefined
  onInstance: (value: number | undefined) => void
  subset: VariableSubset
  onSubset: (value: VariableSubset) => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<VariationState | null>(null)
  const [error, setError] = useState('')
  const configs = useExportProfileStore(
    (store) => store.stored(doc).variableConfigs,
  )
  const [configName, setConfigName] = useState('')
  const [selectedConfig, setSelectedConfig] = useState('')
  const [fileName, setFileName] = useState('')
  const selected = configs.find((config) => config.name === selectedConfig)
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
    void useExportProfileStore
      .getState()
      .load(doc)
      .catch(() => undefined)
    return () => {
      current = false
    }
  }, [doc, t])

  const allMasters = (state?.masters ?? []).map((master) => master.id)
  const allAxes = (state?.axes ?? []).map((axis) => axisTag(axis.tag))
  const allInstances = (state?.instances ?? [])
    .filter((entry) => entry.active)
    .map((entry) => entry.name)
  const chosenMasters = subset.masters ?? allMasters
  const chosenAxes = subset.axes ?? allAxes
  const chosenInstances = subset.instances ?? allInstances

  const toggle = (
    values: readonly string[],
    value: string,
    checked: boolean,
  ): string[] =>
    checked
      ? [...values, value]
      : values.filter((existing) => existing !== value)

  const perform = (action: () => Promise<unknown>) => {
    setError('')
    void action().catch((cause: unknown) => {
      setError(
        cause instanceof ExportProfileError
          ? (cause.detail ?? t(`exportProfiles.errors.${cause.code}`))
          : errorNotice(cause, t).message,
      )
    })
  }

  return (
    <fieldset disabled={busy} className={styles.section}>
      {canVariable ? (
        <label className={styles.row}>
          <input
            type="checkbox"
            checked={variable}
            onChange={(e) => onVariable(e.target.checked)}
          />
          {t('variation.exportVariable')}
        </label>
      ) : null}
      {error && <p role="alert">{error}</p>}
      {state &&
        (variable ? (
          <>
            <fieldset>
              <legend>{t('variation.sourceMasters')}</legend>
              <p className={styles.hint}>{t('variation.subsetHint')}</p>
              {state.masters.map((master) => (
                <label key={master.id} className={styles.checkRow}>
                  <input
                    type="checkbox"
                    checked={chosenMasters.includes(master.id)}
                    onChange={(e) =>
                      onSubset({
                        ...subset,
                        masters: toggle(
                          chosenMasters,
                          master.id,
                          e.target.checked,
                        ),
                      })
                    }
                  />
                  {master.name}
                </label>
              ))}
            </fieldset>
            <fieldset>
              <legend>{t('variation.exportAxes')}</legend>
              <p className={styles.hint}>{t('variation.axisSubsetHint')}</p>
              {state.axes.map((axis) => {
                const tag = axisTag(axis.tag)
                return (
                  <label key={tag} className={styles.checkRow}>
                    <input
                      type="checkbox"
                      checked={chosenAxes.includes(tag)}
                      onChange={(e) =>
                        onSubset({
                          ...subset,
                          axes: toggle(chosenAxes, tag, e.target.checked),
                        })
                      }
                    />
                    {axis.name} ({tag})
                  </label>
                )
              })}
            </fieldset>
            {allInstances.length > 0 ? (
              <fieldset>
                <legend>{t('variation.exportNamedInstances')}</legend>
                <p className={styles.hint}>
                  {t('variation.instanceSubsetHint')}
                </p>
                {allInstances.map((name) => (
                  <label key={name} className={styles.checkRow}>
                    <input
                      type="checkbox"
                      checked={chosenInstances.includes(name)}
                      onChange={(e) =>
                        onSubset({
                          ...subset,
                          instances: toggle(
                            chosenInstances,
                            name,
                            e.target.checked,
                          ),
                        })
                      }
                    />
                    {name}
                  </label>
                ))}
              </fieldset>
            ) : null}
            <fieldset
              className={styles.section}
              onKeyDown={(event) => {
                if (event.key === 'Enter') event.stopPropagation()
              }}
            >
              <legend>{t('variation.exportConfigs')}</legend>
              <p className={styles.hint}>{t('variation.exportConfigsHint')}</p>
              <label className={styles.row}>
                <span className={styles.rowLabel}>
                  {t('variation.configName')}
                </span>
                <input
                  className={styles.text}
                  value={configName}
                  maxLength={80}
                  onChange={(event) => setConfigName(event.target.value)}
                />
              </label>
              <label className={styles.row}>
                <span className={styles.rowLabel}>
                  {t('exportProfiles.fileName')}
                </span>
                <input
                  className={styles.text}
                  value={fileName}
                  maxLength={200}
                  onChange={(event) => setFileName(event.target.value)}
                />
              </label>
              <label className={styles.row}>
                <span className={styles.rowLabel}>
                  {t('variation.storedConfig')}
                </span>
                <select
                  className={styles.text}
                  value={selectedConfig}
                  disabled={configs.length === 0}
                  onChange={(event) => {
                    setSelectedConfig(event.target.value)
                    const chosen = configs.find(
                      (config) => config.name === event.target.value,
                    )
                    if (chosen) setFileName(chosen.file_name)
                  }}
                >
                  <option value="">{t('variation.chooseConfig')}</option>
                  {configs.map((config) => (
                    <option key={config.name} value={config.name}>
                      {config.name}
                    </option>
                  ))}
                </select>
              </label>
              <div className={styles.choices}>
                <Button
                  disabled={!configName.trim()}
                  onClick={() =>
                    perform(async () => {
                      const saved = await useExportProfileStore
                        .getState()
                        .saveConfig(doc, {
                          name: configName,
                          file_name: fileName,
                          masters:
                            chosenMasters.length === allMasters.length
                              ? []
                              : chosenMasters,
                          axes:
                            chosenAxes.length === allAxes.length
                              ? []
                              : chosenAxes,
                          instances:
                            chosenInstances.length === allInstances.length
                              ? []
                              : chosenInstances,
                        })
                      setSelectedConfig(saved.name)
                      setConfigName('')
                    })
                  }
                >
                  {t('variation.saveConfig')}
                </Button>
                <Button
                  disabled={!selected}
                  onClick={() => {
                    if (!selected) return
                    onSubset({
                      masters: selected.masters.length
                        ? selected.masters.filter((id) =>
                            allMasters.includes(id),
                          )
                        : undefined,
                      axes: selected.axes.length ? selected.axes : undefined,
                      instances: selected.instances.length
                        ? selected.instances
                        : undefined,
                    })
                    onVariable(true)
                  }}
                >
                  {t('variation.applyConfig')}
                </Button>
                <Button
                  disabled={!selected}
                  onClick={() =>
                    perform(async () => {
                      if (!selected) return
                      await useExportProfileStore
                        .getState()
                        .removeConfig(doc, selected.name)
                      setSelectedConfig('')
                    })
                  }
                >
                  {t('variation.removeConfig')}
                </Button>
              </div>
            </fieldset>
          </>
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
