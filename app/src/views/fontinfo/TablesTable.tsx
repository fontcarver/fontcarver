import { useTranslation } from 'react-i18next'

import type { TableInfo } from '../../api'

import styles from './fontInfo.module.css'
import { rowMatches } from './fontInfoRows'
import { SectionBody } from './SectionBody'

export interface TablesTableProps {
  tables: readonly TableInfo[]
  /** Search term applied to the tag, length and preservation marker. */
  query: string
  heading?: string
}

/** Imported OpenType table directory, including verbatim-preservation status. */
export function TablesTable({ tables, query, heading }: TablesTableProps) {
  const { t } = useTranslation()
  const preserved = t('fontInfo.preserved')
  const rows = [...tables]
    .sort((left, right) => left.tag.localeCompare(right.tag, 'en'))
    .filter((table) =>
      rowMatches(
        {
          label: table.tag,
          value: `${table.len} ${table.preserved ? preserved : ''}`,
        },
        query,
      ),
    )

  return (
    <SectionBody
      heading={heading}
      hasContent={rows.length > 0}
      empty={
        tables.length === 0 ? t('fontInfo.noTables') : t('fontInfo.noMatches')
      }
    >
      <div className={styles.tableScroll}>
        <table className={styles.table} aria-label={t('fontInfo.tablesLabel')}>
          <thead>
            <tr>
              <th scope="col">{t('fontInfo.column.tag')}</th>
              <th scope="col">{t('fontInfo.column.byteLength')}</th>
              <th scope="col">{t('fontInfo.column.preservation')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((table) => (
              <tr key={table.tag}>
                <td>{table.tag}</td>
                <td className={styles.numeric}>
                  {t('fontInfo.bytes', { count: table.len })}
                </td>
                <td>
                  {table.preserved ? (
                    <span className={styles.preserved}>{preserved}</span>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SectionBody>
  )
}
