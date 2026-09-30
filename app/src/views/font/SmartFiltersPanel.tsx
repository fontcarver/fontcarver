import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId } from '../../api'
import { Button } from '../../components/Button'
import { Panel } from '../../components/Panel'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { FilterRow } from './FilterRow'
import { selectQueryError, useQueryErrors } from './queryErrors'
import { SmartFilterEditor } from './SmartFilterEditor'
import { useSmartFilterCounts } from './smartFilterCounts'
import { useSmartFilters, type SavedSmartFilter } from './useSmartFilters'
import styles from './smartFiltersPanel.module.css'

export interface SmartFiltersPanelProps {
  doc: DocId
  expression: string | null
  /** Glyphs in the document, the denominator of a filter's match badge. */
  totalGlyphs?: number
  onExpressionChange: (expression: string | null) => void
}

/** Project-persisted filters, with selection and undoable CRUD actions. */
export function SmartFiltersPanel({
  doc,
  expression,
  totalGlyphs,
  onExpressionChange,
}: SmartFiltersPanelProps) {
  const { t } = useTranslation()
  const { filters, loaded, busy, error, save, retry } = useSmartFilters(doc)
  const generation = useDocumentStore(selectDocView(doc)).generation
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
  const expressions = useMemo(
    () => filters.map((filter) => filter.expression),
    [filters],
  )
  const counts = useSmartFilterCounts(doc, expressions, generation)
  const queryError = useQueryErrors(selectQueryError(doc, 'smartFilter'))
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
        {filters.length === 0 ? (
          loaded && !error ? (
            <p className={styles.hint}>{t('smartFilters.none')}</p>
          ) : null
        ) : (
          <div
            className={styles.list}
            role="group"
            aria-label={t('smartFilters.savedFilters')}
          >
            {filters.map((filter) => (
              <FilterRow
                key={filter.name}
                label={filter.name}
                count={counts.get(filter.expression)}
                total={counts.has(filter.expression) ? totalGlyphs : undefined}
                active={filter.expression === expression}
                title={filter.expression}
                onToggle={() =>
                  onExpressionChange(
                    filter.expression === expression ? null : filter.expression,
                  )
                }
              />
            ))}
          </div>
        )}
        {expression !== null && selected < 0 && (
          <p className={styles.hint}>{t('smartFilters.customFilter')}</p>
        )}
        {!loaded && !error && <p role="status">{t('smartFilters.loading')}</p>}
        {error && (
          <p className={styles.error} role="alert">
            {t(`smartFilters.${error}`)}
          </p>
        )}
        {queryError !== null && (
          <p className={styles.error} role="alert">
            {t('glyphSearch.invalidQuery', {
              message: queryError.replace(/\s+/gu, ' ').trim(),
            })}
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
