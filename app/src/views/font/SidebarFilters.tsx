import { Fragment, useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId, RepertoireKind } from '../../api'
import { Panel } from '../../components/Panel'
import panelStyles from '../../shell/panels.module.css'
import {
  emptyFilter,
  isFiltered,
  selectDocView,
  useDocumentStore,
} from '../../stores/documentStore'

import { COLOR_LABELS, colorLabelKey } from './colorLabels'
import { CjkFilters } from './CjkFilters'
import { useCoverage, type CoverageIndex } from './coverage'
import { FilterRow } from './FilterRow'
import { GenerateMissing } from './GenerateMissing'
import { GlyphStatusFilters } from './GlyphStatusFilters'
import { LanguageFilters } from './LanguageFilters'
import { GlyphListsPanel } from './GlyphListsPanel'
import { SmartFiltersPanel } from './SmartFiltersPanel'
import languageStyles from './languageFilters.module.css'

export interface SidebarFiltersProps {
  doc: DocId
}

/** Delay between an edit and the taxonomy-count refresh, in milliseconds. */
export const SIDEBAR_REFRESH_MS = 250

/** Adds or removes one value of a multi-select filter section. */
function toggled<T>(values: readonly T[], value: T): T[] {
  return values.includes(value)
    ? values.filter((entry) => entry !== value)
    : [...values, value]
}

/**
 * One sectioned row: its present/total badge and its "generate missing" button.
 *
 * `present` counts the repertoire's characters the font encodes and `total` its
 * assigned Unicode scalars, so the badge reads as coverage and the button has a
 * definite set to create. The glyph count the filter itself selects is in the
 * row's tooltip, because it answers a different question. Coverage that has not
 * arrived yet leaves the row with its plain glyph count.
 */
function SectionRow({
  doc,
  kind,
  value,
  label,
  glyphs,
  coverage,
  active,
  onToggle,
}: {
  doc: DocId
  kind: RepertoireKind
  value: string
  label: string
  glyphs: number
  coverage: CoverageIndex | null
  active: boolean
  onToggle: () => void
}) {
  const { t } = useTranslation()
  const row = coverage?.get(value)
  const missing = row ? Math.max(0, row.total - row.present) : 0
  return (
    <div className={languageStyles.row}>
      <FilterRow
        label={label}
        count={row?.present ?? glyphs}
        total={row?.total}
        active={active}
        title={
          row
            ? t('sidebar.coverageDetail', {
                glyphs,
                present: row.present,
                total: row.total,
                missing,
              })
            : undefined
        }
        onToggle={onToggle}
      />
      {row && missing > 0 ? (
        <GenerateMissing
          doc={doc}
          kind={kind}
          value={value}
          name={label}
          missing={missing}
        />
      ) : null}
    </div>
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
  const stale = view.sidebarGeneration !== view.generation
  const coverage = useCoverage(doc, view.generation)

  useEffect(() => {
    if (!stale) return
    // The first load has nothing to show, so it goes out at once. Later the
    // counts are derived from every glyph, so an edit invalidates them: the
    // previous counts stay on screen until the new ones land, and a burst of
    // edits collapses into one request.
    if (counts === null) {
      void loadSidebar(doc)
      return
    }
    const timer = setTimeout(() => void loadSidebar(doc), SIDEBAR_REFRESH_MS)
    return () => clearTimeout(timer)
  }, [counts, doc, loadSidebar, stale])

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

      <GlyphListsPanel doc={doc} />
      <SmartFiltersPanel
        doc={doc}
        expression={filter.smart_expression ?? null}
        totalGlyphs={counts.total}
        onExpressionChange={(smart_expression) =>
          setFilter(doc, { smart_expression })
        }
      />
      <GlyphStatusFilters
        doc={doc}
        expression={filter.smart_expression ?? null}
      />

      {(counts.scripts.some(([script]) => script === 'Hani') || filter.cjk) && (
        <CjkFilters doc={doc} value={filter.cjk} />
      )}
      {counts.categories.length > 0 ? (
        <Panel title={t('sidebar.categories')}>
          {counts.categories.map(([name, count]) => (
            <Fragment key={name}>
              <SectionRow
                doc={doc}
                kind="Category"
                value={name}
                label={t(`sidebar.categoryNames.${name}`, {
                  defaultValue: name,
                })}
                glyphs={count}
                coverage={coverage?.categories ?? null}
                active={filter.categories.includes(name)}
                onToggle={() =>
                  setFilter(doc, {
                    categories: toggled(filter.categories, name).filter(
                      (value) => !value.startsWith(`${name}/`),
                    ),
                  })
                }
              />
              {(counts.sub_categories ?? [])
                .filter(([category]) => category === name)
                .map(([, subcategory, subcount]) => {
                  const value = `${name}/${subcategory}`
                  return (
                    <div key={value} className={languageStyles.subcategory}>
                      <SectionRow
                        doc={doc}
                        kind="SubCategory"
                        value={value}
                        label={t(`sidebar.categoryNames.${subcategory}`, {
                          defaultValue: subcategory,
                        })}
                        glyphs={subcount}
                        coverage={coverage?.subCategories ?? null}
                        active={filter.categories.includes(value)}
                        onToggle={() =>
                          setFilter(doc, {
                            categories: toggled(
                              filter.categories.filter(
                                (entry) => entry !== name,
                              ),
                              value,
                            ),
                          })
                        }
                      />
                    </div>
                  )
                })}
            </Fragment>
          ))}
        </Panel>
      ) : null}

      {counts.unicodes && (
        <LanguageFilters doc={doc} filter={filter} unicodes={counts.unicodes} />
      )}

      {counts.scripts.length > 0 ? (
        <Panel title={t('sidebar.scripts')}>
          {counts.scripts.map(([name, count]) => (
            <SectionRow
              key={name}
              doc={doc}
              kind="Script"
              value={name}
              label={name}
              glyphs={count}
              coverage={coverage?.scripts ?? null}
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
            <SectionRow
              key={name}
              doc={doc}
              kind="Block"
              value={name}
              label={name}
              glyphs={count}
              coverage={coverage?.blocks ?? null}
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
