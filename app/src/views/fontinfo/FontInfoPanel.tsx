import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { onDocumentEvent, type DocSummary } from '../../api'
import { Toolbar } from '../../components/Toolbar'
import type { WarningTranslationOptions } from '../../i18n/warnings'
import { useFontInfo } from '../../stores/fontInfoCache'
import { invalidateFor } from '../font/invalidation'

import { AxesTable } from './AxesTable'
import styles from './fontInfo.module.css'
import { fontGroups, otherGroups } from './fontInfoRows'
import { InfoGroups } from './InfoGroups'
import { MetricsTable } from './MetricsTable'
import { NamesTable } from './NamesTable'
import { TablesTable } from './TablesTable'

/** The sections of the Font Info tab, in order. */
export const SECTIONS = [
  'font',
  'names',
  'masters',
  'axes',
  'tables',
  'other',
] as const

/** One Font Info section. */
export type SectionId = (typeof SECTIONS)[number]

export interface FontInfoPanelProps {
  summary: DocSummary
}

/**
 * The Font Info tab: everything the kernel knows about the font, read-only.
 *
 * A search box filters fields across *every* section at once — the single
 * biggest quality-of-life gap in the incumbent editors, where finding
 * `usWeightClass` means knowing which tab hides it.
 */
export function FontInfoPanel({ summary }: FontInfoPanelProps) {
  const { t } = useTranslation()
  const [section, setSection] = useState<SectionId>('font')
  const [query, setQuery] = useState('')
  const { info, error, loading } = useFontInfo(summary.doc)

  // The panel is the only view mounted while it is the active tab, so it is
  // the one that has to hand document events to the shared invalidation; the
  // reload itself comes back through the font-info cache.
  useEffect(
    () =>
      onDocumentEvent((event) => {
        if (event.doc === summary.doc)
          invalidateFor(event.change, event.doc, event.generation)
      }),
    [summary.doc],
  )

  const searching = query.trim() !== ''

  const groups = useMemo(() => {
    const translate = (key: string, options?: WarningTranslationOptions) =>
      options ? t(key, options) : t(key)
    return info === null
      ? { font: [], other: [] }
      : {
          font: fontGroups(info.info, summary, translate),
          other: otherGroups(info.info, translate),
        }
  }, [info, summary, t])

  if (error) return <p className={styles.empty}>{error}</p>
  if (!info)
    return (
      <p className={styles.empty}>
        {loading ? t('fontInfo.loading') : t('fontInfo.unavailable')}
      </p>
    )

  const shown: readonly SectionId[] = searching ? SECTIONS : [section]

  const renderSection = (id: SectionId) => {
    // While searching, every section is rendered and each shows its own name;
    // sections with no matches render nothing at all.
    const heading = searching ? t(`fontInfo.section.${id}`) : undefined
    switch (id) {
      case 'font':
        return (
          <InfoGroups
            heading={heading}
            groups={groups.font}
            query={query}
            empty={t('fontInfo.noMatches')}
          />
        )
      case 'names':
        return (
          <NamesTable heading={heading} names={info.info.names} query={query} />
        )
      case 'masters':
        return (
          <MetricsTable
            heading={heading}
            masters={info.masters}
            query={query}
          />
        )
      case 'axes':
        return (
          <AxesTable
            heading={heading}
            axes={summary.axes}
            masters={info.masters}
            instances={info.instances}
            query={query}
          />
        )
      case 'tables':
        return (
          <TablesTable heading={heading} tables={info.tables} query={query} />
        )
      case 'other':
        return (
          <InfoGroups
            heading={heading}
            groups={groups.other}
            query={query}
            empty={t('fontInfo.noMatches')}
          />
        )
    }
  }

  return (
    <div className={styles.view}>
      <Toolbar label={t('fontInfo.toolbar')} className={styles.toolbar}>
        <span role="tablist" aria-label={t('fontInfo.sections')}>
          {SECTIONS.map((id) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={!searching && section === id}
              className={styles.sectionTab}
              onClick={() => {
                setSection(id)
                setQuery('')
              }}
            >
              {t(`fontInfo.section.${id}`)}
            </button>
          ))}
        </span>
        <span className={styles.toolbarSpacer} />
        <input
          type="search"
          className={styles.search}
          aria-label={t('fontInfo.search')}
          placeholder={t('fontInfo.searchPlaceholder')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </Toolbar>

      <div className={styles.body}>
        {shown.map((id) => (
          <section key={id} aria-label={t(`fontInfo.section.${id}`)}>
            {renderSection(id)}
          </section>
        ))}
      </div>
    </div>
  )
}
