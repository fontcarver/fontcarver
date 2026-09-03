import styles from './fontInfo.module.css'
import type { InfoGroup } from './fontInfoRows'
import { filterGroups } from './fontInfoRows'
import { SectionBody } from './SectionBody'

export interface InfoGroupsProps {
  groups: readonly InfoGroup[]
  /** Search term; empty shows everything. */
  query: string
  empty: string
  heading?: string
}

/** Renders label/value groups, dropping rows that do not match the search. */
export function InfoGroups({ groups, query, empty, heading }: InfoGroupsProps) {
  const visible = filterGroups(groups, query)

  return (
    <SectionBody
      heading={heading}
      hasContent={visible.length > 0}
      empty={empty}
    >
      {visible.map((group) => (
        <section key={group.id} className={styles.group}>
          <h3 className={styles.groupTitle}>{group.title}</h3>
          <dl className={styles.rows}>
            {group.rows.map((row) => (
              <div key={`${group.id}:${row.label}`} className={styles.row}>
                <dt className={styles.rowLabel}>{row.label}</dt>
                <dd className={styles.rowValue}>{row.value}</dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
    </SectionBody>
  )
}
