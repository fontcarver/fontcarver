import { useTranslation } from 'react-i18next'

import type { DocId } from '../../api'
import { Panel } from '../../components/Panel'
import { useDocumentStore } from '../../stores/documentStore'

import { FilterRow } from './FilterRow'

/** One badge and the smart expression that selects the glyphs carrying it. */
const BADGE_FILTERS = [
  ['metricsOutOfSync', 'metrics_out_of_sync:"true"'],
  ['incompatibleMasters', 'master_compatible:"false"'],
  ['placeholder', 'is_placeholder:"true"'],
  ['annotations', 'has_annotations:"true"'],
  ['customGlyphInfo', 'has_custom_glyph_info:"true"'],
  ['notExporting', 'export:"false"'],
  ['locked', 'locked:"true"'],
] as const

export interface GlyphStatusFiltersProps {
  doc: DocId
  /** The smart expression currently in the filter, if any. */
  expression: string | null
}

/**
 * One click per cell badge.
 *
 * Every badge the grid paints has a smart-filter predicate behind it, so "show
 * me the glyphs whose metrics are out of sync" is a toggle rather than an
 * expression the person has to remember how to write. Selecting a row replaces
 * the smart expression, which is exactly what the saved-filter list does, so the
 * two never disagree about what is active.
 */
export function GlyphStatusFilters({
  doc,
  expression,
}: GlyphStatusFiltersProps) {
  const { t } = useTranslation()
  const setFilter = useDocumentStore((state) => state.setFilter)

  return (
    <Panel title={t('sidebar.statuses')}>
      {BADGE_FILTERS.map(([key, smart]) => (
        <FilterRow
          key={key}
          label={t(`fontView.glyphStatus.${key}`)}
          active={expression === smart}
          onToggle={() =>
            setFilter(doc, {
              smart_expression: expression === smart ? null : smart,
            })
          }
        />
      ))}
    </Panel>
  )
}
