import { useTranslation } from 'react-i18next'

import type { NameRecord } from '../../api'

import styles from './fontInfo.module.css'
import { nameIdKey } from './nameIds'
import { rowMatches } from './fontInfoRows'
import { SectionBody } from './SectionBody'

export interface NamesTableProps {
  names: readonly NameRecord[]
  /** Search term applied to the id label, language and text. */
  query: string
  heading?: string
}

/**
 * Every imported `name` record, including the non-default languages that the
 * flattened fields above cannot show. This is the table that answers "why does
 * this font show a different name on Windows?".
 */
export function NamesTable({ names, query, heading }: NamesTableProps) {
  const { t } = useTranslation()

  const rows = names.filter((record) => {
    const key = nameIdKey(record.name_id)
    const label = key ? t(key) : ''
    return rowMatches(
      {
        label: `${record.name_id} ${label} ${record.language} ${record.platform}`,
        value: record.text,
      },
      query,
    )
  })

  return (
    <SectionBody
      heading={heading}
      hasContent={rows.length > 0}
      empty={
        query.trim() === '' ? t('fontInfo.noNames') : t('fontInfo.noMatches')
      }
    >
      <div className={styles.tableScroll}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th scope="col">{t('fontInfo.column.nameId')}</th>
              <th scope="col">{t('fontInfo.column.nameLabel')}</th>
              <th scope="col">{t('fontInfo.column.platform')}</th>
              <th scope="col">{t('fontInfo.column.language')}</th>
              <th scope="col">{t('fontInfo.column.text')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((record, index) => {
              const key = nameIdKey(record.name_id)
              return (
                <tr
                  key={`${record.name_id}-${record.platform}-${record.language}-${index}`}
                >
                  <td className={styles.numeric}>{record.name_id}</td>
                  <td>{key ? t(key) : '—'}</td>
                  <td>{record.platform}</td>
                  <td>{record.language}</td>
                  <td className={styles.wrap}>{record.text}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </SectionBody>
  )
}
