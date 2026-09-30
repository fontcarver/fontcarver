import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId } from '../../api'
import { Button } from '../../components/Button'
import { Panel } from '../../components/Panel'
import { SmartFilterEditor } from './SmartFilterEditor'
import { useSmartFilters, type SavedSmartFilter } from './useSmartFilters'
import styles from './smartFiltersPanel.module.css'

export interface SmartFiltersPanelProps {
  doc: DocId
  expression: string | null
  onExpressionChange: (expression: string | null) => void
}

/** Project-persisted filters, with selection and undoable CRUD actions. */
export function SmartFiltersPanel({
  doc,
  expression,
  onExpressionChange,
}: SmartFiltersPanelProps) {
  const { t } = useTranslation()
  const { filters, loaded, busy, error, save, retry } = useSmartFilters(doc)
  const [editing, setEditing] = useState<{
    doc: DocId
    filter?: SavedSmartFilter
  } | null>(null)
  const currentEdit = editing?.doc === doc ? editing : null
  const selected = filters.findIndex(
    (filter) => filter.expression === expression,
  )
  const selectedFilter = filters[selected]
  const disabled = !loaded || busy || error === 'loadError'
  const commit = async (filter: SavedSmartFilter) => {
    const original = currentEdit?.filter
    if (original && !filters.some((entry) => entry.name === original.name)) {
      throw new Error(t('smartFilters.saveError'))
    }
    const next = original
      ? filters.map((entry) => (entry.name === original.name ? filter : entry))
      : [...filters, filter]
    if (await save(next)) {
      setEditing(null)
      onExpressionChange(filter.expression)
    }
  }
  const remove = async () => {
    if (!selectedFilter) return
    if (
      await save(
        filters.filter((filter) => filter.name !== selectedFilter.name),
      )
    ) {
      onExpressionChange(null)
    }
  }

  return (
    <Panel title={t('smartFilters.title')}>
      <div className={styles.content}>
        <label className={styles.choice}>
          {t('smartFilters.savedFilters')}
          <select
            disabled={disabled}
            value={
              selected >= 0 ? String(selected) : expression ? 'custom' : ''
            }
            onChange={(event) => {
              const value = event.target.value
              onExpressionChange(
                value === '' ? null : filters[Number(value)].expression,
              )
            }}
          >
            <option value="">{t('smartFilters.noFilter')}</option>
            {expression && selected < 0 && (
              <option value="custom" disabled>
                {t('smartFilters.customFilter')}
              </option>
            )}
            {filters.map((filter, index) => (
              <option key={filter.name} value={index}>
                {filter.name}
              </option>
            ))}
          </select>
        </label>
        {!loaded && !error && <p role="status">{t('smartFilters.loading')}</p>}
        {error && (
          <p className={styles.error} role="alert">
            {t(`smartFilters.${error}`)}
          </p>
        )}
        <div className={styles.actions}>
          {error === 'loadError' && (
            <Button disabled={busy} onClick={retry}>
              {t('smartFilters.retry')}
            </Button>
          )}
          <Button disabled={disabled} onClick={() => setEditing({ doc })}>
            {t('smartFilters.new')}
          </Button>
          <Button
            disabled={disabled || !selectedFilter}
            onClick={() => setEditing({ doc, filter: selectedFilter })}
          >
            {t('smartFilters.edit')}
          </Button>
          <Button
            disabled={disabled || !selectedFilter}
            onClick={() => {
              void remove().catch(() => undefined)
            }}
          >
            {t('smartFilters.delete')}
          </Button>
          <Button
            disabled={busy || expression === null}
            onClick={() => onExpressionChange(null)}
          >
            {t('smartFilters.clear')}
          </Button>
        </div>
      </div>
      {currentEdit && (
        <SmartFilterEditor
          key={`${doc}:${currentEdit.filter?.name ?? ''}`}
          initialFilter={currentEdit.filter}
          onSave={commit}
          onCancel={() => setEditing(null)}
        />
      )}
    </Panel>
  )
}
