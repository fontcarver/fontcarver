import { useTranslation } from 'react-i18next'

import type { AxisSummary, Master } from '../../api'

import styles from './fontInfo.module.css'
import { rowMatches } from './fontInfoRows'
import { SectionBody } from './SectionBody'

export interface AxesTableProps {
  axes: readonly AxisSummary[]
  masters: readonly Master[]
  /** Search term applied to the axis tag and name. */
  query: string
  heading?: string
}

/**
 * The design space: the variation axes and where each master sits on them.
 * Named instances are not part of the kernel's document model yet, so this
 * section stops at masters.
 */
export function AxesTable({ axes, masters, query, heading }: AxesTableProps) {
  const { t } = useTranslation()
  const visible = axes.filter((axis) =>
    rowMatches({ label: axis.tag, value: axis.name }, query),
  )

  return (
    <SectionBody
      heading={heading}
      hasContent={visible.length > 0}
      empty={axes.length === 0 ? t('fontInfo.noAxes') : t('fontInfo.noMatches')}
    >
      <div className={styles.tableScroll}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th scope="col">{t('fontInfo.column.tag')}</th>
              <th scope="col">{t('fontInfo.column.axisName')}</th>
              <th scope="col">{t('fontInfo.column.min')}</th>
              <th scope="col">{t('fontInfo.column.default')}</th>
              <th scope="col">{t('fontInfo.column.max')}</th>
              <th scope="col">{t('fontInfo.column.hidden')}</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((axis) => (
              <tr key={axis.tag}>
                <td>{axis.tag}</td>
                <td>{axis.name}</td>
                <td className={styles.numeric}>{axis.min}</td>
                <td className={styles.numeric}>{axis.default}</td>
                <td className={styles.numeric}>{axis.max}</td>
                <td>{axis.hidden ? t('common.yes') : t('common.no')}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className={styles.tableScroll}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th scope="col">{t('fontInfo.column.master')}</th>
              {visible.map((axis) => (
                <th key={axis.tag} scope="col">
                  {axis.tag}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {masters.map((master) => (
              <tr key={master.id.toString()}>
                <th scope="row">{master.name}</th>
                {visible.map((axis) => (
                  <td key={axis.tag} className={styles.numeric}>
                    {master.location[axis.tag] ?? '—'}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SectionBody>
  )
}
