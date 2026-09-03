import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId, GlyphFilter } from '../../api'
import { Panel } from '../../components/Panel'
import {
  emptyFilter,
  selectDocView,
  useDocumentStore,
} from '../../stores/documentStore'
import panelStyles from '../../shell/panels.module.css'

import { COLOR_LABELS, colorLabelKey } from './colorLabels'
import { FilterRow } from './FilterRow'

export interface SidebarFiltersProps {
  doc: DocId
}

/** Adds or removes one value of a multi-select filter section. */
function toggled<T>(values: readonly T[], value: T): T[] {
  return values.includes(value)
    ? values.filter((entry) => entry !== value)
    : [...values, value]
}

/** Whether any condition of the filter is set. */
export function isFiltered(filter: GlyphFilter): boolean {
  return (
    filter.query !== null ||
    filter.categories.length > 0 ||
    filter.scripts.length > 0 ||
    filter.blocks.length > 0 ||
    filter.color_labels.length > 0 ||
    filter.tags.length > 0 ||
    filter.unicode_ranges.length > 0 ||
    filter.unencoded_only ||
    filter.changed_only ||
    filter.has_components !== null
  )
}

/**
 * The font view's sidebar: All, Categories, Scripts, Unicode blocks, Colours
 * and Unencoded. Values inside a section are OR-ed, sections are AND-ed, which
 * is exactly what the kernel's `GlyphFilter` does.
 */
export function SidebarFilters({ doc }: SidebarFiltersProps) {
  const { t } = useTranslation()
  const view = useDocumentStore(selectDocView(doc))
  const loadSidebar = useDocumentStore((state) => state.loadSidebar)
  const setFilter = useDocumentStore((state) => state.setFilter)
  const counts = view.sidebar
  const filter = view.filter

  useEffect(() => {
    if (counts === null) void loadSidebar(doc)
  }, [counts, doc, loadSidebar])

  if (counts === null)
    return <p className={panelStyles.hintBlock}>{t('sidebar.empty')}</p>

  const colorCounts = new Map(counts.color_labels)

  return (
    <>
      <Panel title={t('sidebar.overview')}>
        <FilterRow
          label={t('sidebar.all')}
          count={counts.total}
          active={!isFiltered(filter)}
          onToggle={() => setFilter(doc, emptyFilter())}
        />
        <FilterRow
          label={t('sidebar.unencoded')}
          count={counts.unencoded}
          active={filter.unencoded_only}
          onToggle={() =>
            setFilter(doc, {
              unencoded_only: !filter.unencoded_only,
              unicode_ranges: [],
            })
          }
        />
      </Panel>

      {counts.categories.length > 0 ? (
        <Panel title={t('sidebar.categories')}>
          {counts.categories.map(([name, count]) => (
            <FilterRow
              key={name}
              label={name}
              count={count}
              active={filter.categories.includes(name)}
              onToggle={() =>
                setFilter(doc, { categories: toggled(filter.categories, name) })
              }
            />
          ))}
        </Panel>
      ) : null}

      {counts.scripts.length > 0 ? (
        <Panel title={t('sidebar.scripts')}>
          {counts.scripts.map(([name, count]) => (
            <FilterRow
              key={name}
              label={name}
              count={count}
              active={filter.scripts.includes(name)}
              onToggle={() =>
                setFilter(doc, { scripts: toggled(filter.scripts, name) })
              }
            />
          ))}
        </Panel>
      ) : null}

      {counts.blocks.length > 0 ? (
        <Panel title={t('sidebar.blocks')}>
          {counts.blocks.map(([name, count]) => (
            <FilterRow
              key={name}
              label={name}
              count={count}
              active={filter.blocks.includes(name)}
              onToggle={() =>
                setFilter(doc, { blocks: toggled(filter.blocks, name) })
              }
            />
          ))}
        </Panel>
      ) : null}

      {colorCounts.size > 0 ? (
        <Panel title={t('sidebar.colors')}>
          {COLOR_LABELS.filter((label) => colorCounts.has(label.index)).map(
            (label) => (
              <FilterRow
                key={label.index}
                label={t(`fontView.color.${colorLabelKey(label.index)}`)}
                count={colorCounts.get(label.index) ?? 0}
                swatch={label.swatch}
                active={filter.color_labels.includes(label.index)}
                onToggle={() =>
                  setFilter(doc, {
                    color_labels: toggled(filter.color_labels, label.index),
                  })
                }
              />
            ),
          )}
        </Panel>
      ) : null}
    </>
  )
}
