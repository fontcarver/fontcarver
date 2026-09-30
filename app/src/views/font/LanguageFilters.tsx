import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { DocId, GlyphFilter } from '../../api'
import { Panel } from '../../components/Panel'
import { emptyFilter, useDocumentStore } from '../../stores/documentStore'
import { useUiStore } from '../../stores/uiStore'
import { FilterRow } from './FilterRow'
import {
  languageCoverage,
  languageRepertoires,
  scalarRanges,
  scalarRecipe,
} from './languageRepertoires'
import styles from './languageFilters.module.css'

/** Letter coverage for bounded language repertoires, independent of shaping. */
export function LanguageFilters({
  doc,
  filter,
  unicodes,
}: {
  doc: DocId
  filter: GlyphFilter
  unicodes: readonly number[]
}) {
  const { t } = useTranslation()
  const setFilter = useDocumentStore((state) => state.setFilter)
  const setDialog = useUiStore((state) => state.setDialog)
  const coverages = useMemo(
    () =>
      languageRepertoires.map((language) =>
        languageCoverage(language.scalars, unicodes),
      ),
    [unicodes],
  )
  return (
    <Panel title={t('sidebar.languages')}>
      <p className={styles.hint}>{t('sidebar.languageHint')}</p>
      {languageRepertoires.map((language, index) => {
        const ranges = scalarRanges(language.scalars)
        const coverage = coverages[index]
        const active =
          JSON.stringify(filter.unicode_ranges) === JSON.stringify(ranges)
        const name = t(`sidebar.languageNames.${language.id}`)
        return (
          <div className={styles.row} key={language.id}>
            <FilterRow
              label={name}
              count={coverage.present}
              total={coverage.total}
              active={active}
              onToggle={() =>
                setFilter(doc, {
                  ...emptyFilter(),
                  unicode_ranges: active ? [] : ranges,
                })
              }
            />
            <button
              type="button"
              className={styles.generate}
              disabled={coverage.missing.length === 0}
              title={t('sidebar.generateMissing', {
                name,
                count: coverage.missing.length,
              })}
              aria-label={t('sidebar.generateMissing', {
                name,
                count: coverage.missing.length,
              })}
              onClick={() =>
                setDialog({
                  kind: 'addGlyphs',
                  doc,
                  recipe: scalarRecipe(coverage.missing),
                })
              }
            >
              +
            </button>
          </div>
        )
      })}
    </Panel>
  )
}
