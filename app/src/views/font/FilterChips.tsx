import { useTranslation } from 'react-i18next'

import type { DocId, GlyphFilter } from '../../api'
import { emptyFilter, useDocumentStore } from '../../stores/documentStore'

import { colorLabelKey } from './colorLabels'
import styles from './fontView.module.css'

export interface FilterChipsProps {
  doc: DocId
  filter: GlyphFilter
}

interface Chip {
  key: string
  label: string
  patch: Partial<GlyphFilter>
}

function withoutValue<T>(values: readonly T[], value: T): T[] {
  return values.filter((entry) => entry !== value)
}

/** The chips describing the filter conditions currently in force. */
export function filterChips(
  filter: GlyphFilter,
  label: (key: string, options?: Record<string, unknown>) => string,
): Chip[] {
  const chips: Chip[] = []
  if (filter.query)
    chips.push({
      key: 'query',
      label: label('fontView.chip.query', { query: filter.query }),
      patch: { query: null },
    })
  for (const category of filter.categories)
    chips.push({
      key: `category:${category}`,
      label: category,
      patch: { categories: withoutValue(filter.categories, category) },
    })
  for (const script of filter.scripts)
    chips.push({
      key: `script:${script}`,
      label: script,
      patch: { scripts: withoutValue(filter.scripts, script) },
    })
  for (const block of filter.blocks)
    chips.push({
      key: `block:${block}`,
      label: block,
      patch: { blocks: withoutValue(filter.blocks, block) },
    })
  for (const color of filter.color_labels)
    chips.push({
      key: `color:${color}`,
      label: label(`fontView.color.${colorLabelKey(color)}`),
      patch: { color_labels: withoutValue(filter.color_labels, color) },
    })
  if (filter.unencoded_only)
    chips.push({
      key: 'unencoded',
      label: label('sidebar.unencoded'),
      patch: { unencoded_only: false },
    })
  if (filter.unicode_ranges.length > 0)
    chips.push({
      key: 'encoded',
      label: label('fontView.encodedOnly'),
      patch: { unicode_ranges: [] },
    })
  return chips
}

/** Removable chips above the grid, one per active filter condition. */
export function FilterChips({ doc, filter }: FilterChipsProps) {
  const { t } = useTranslation()
  const setFilter = useDocumentStore((state) => state.setFilter)
  const chips = filterChips(filter, (key, options) => t(key, options ?? {}))

  if (chips.length === 0) return null

  return (
    <div className={styles.chips}>
      {chips.map((chip) => (
        <button
          key={chip.key}
          type="button"
          className={styles.chip}
          aria-label={t('fontView.removeFilter', { name: chip.label })}
          onClick={() => setFilter(doc, chip.patch)}
        >
          {chip.label}
          <span aria-hidden="true" className={styles.chipRemove}>
            ✕
          </span>
        </button>
      ))}
      <button
        type="button"
        className={styles.chipClear}
        onClick={() => setFilter(doc, emptyFilter())}
      >
        {t('fontView.clearFilters')}
      </button>
    </div>
  )
}
