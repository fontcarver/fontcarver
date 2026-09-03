import { useTranslation } from 'react-i18next'

import type { AxisSummary, Instance, Master } from '../../api'

import styles from './fontInfo.module.css'
import { rowMatches } from './fontInfoRows'
import { SectionBody } from './SectionBody'

export interface AxesTableProps {
  axes: readonly AxisSummary[]
  masters: readonly Master[]
  instances: readonly Instance[]
  /** Search term applied to axes, locations and instance metadata. */
  query: string
  heading?: string
}

/**
 * The design space: axes, master locations, and named export instances.
 */
export function AxesTable({
  axes,
  masters,
  instances,
  query,
  heading,
}: AxesTableProps) {
  const { t } = useTranslation()
  const searching = query.trim() !== ''
  const visibleAxes = axes.filter((axis) =>
    rowMatches({ label: axis.tag, value: axis.name }, query),
  )
  const visibleInstances = instances.filter((instance) =>
    rowMatches(
      {
        label: [
          instance.name,
          instance.style_name,
          instance.family_name ?? '',
          instance.active ? t('common.yes') : t('common.no'),
          instance.is_variable_named_instance
            ? t('common.yes')
            : t('common.no'),
        ].join(' '),
        value: Object.entries(instance.location)
          .map(([tag, value]) => `${tag} ${value}`)
          .join(' '),
      },
      query,
    ),
  )
  const hasContent =
    !searching || visibleAxes.length > 0 || visibleInstances.length > 0

  return (
    <SectionBody
      heading={heading}
      hasContent={hasContent}
      empty={t('fontInfo.noMatches')}
    >
      {visibleAxes.length > 0 ? (
        <>
          <h3 className={styles.subsectionTitle}>{t('fontInfo.group.axes')}</h3>
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
                {visibleAxes.map((axis) => (
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

          {masters.length > 0 ? (
            <>
              <h3 className={styles.subsectionTitle}>
                {t('fontInfo.group.masterLocations')}
              </h3>
              <div className={styles.tableScroll}>
                <table className={styles.table}>
                  <thead>
                    <tr>
                      <th scope="col">{t('fontInfo.column.master')}</th>
                      {visibleAxes.map((axis) => (
                        <th key={axis.tag} scope="col">
                          {axis.tag}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {masters.map((master) => (
                      <tr key={master.id}>
                        <th scope="row">{master.name}</th>
                        {visibleAxes.map((axis) => (
                          <td key={axis.tag} className={styles.numeric}>
                            {master.location[axis.tag] ?? '—'}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          ) : null}
        </>
      ) : !searching && axes.length === 0 ? (
        <p className={styles.empty}>{t('fontInfo.noAxes')}</p>
      ) : null}

      {visibleInstances.length > 0 ? (
        <>
          <h3 className={styles.subsectionTitle}>
            {t('fontInfo.group.instances')}
          </h3>
          <div className={styles.tableScroll}>
            <table
              className={styles.table}
              aria-label={t('fontInfo.group.instances')}
            >
              <thead>
                <tr>
                  <th scope="col">{t('fontInfo.column.instanceName')}</th>
                  <th scope="col">{t('fontInfo.column.styleName')}</th>
                  <th scope="col">{t('fontInfo.column.familyOverride')}</th>
                  {axes.map((axis) => (
                    <th key={axis.tag} scope="col">
                      {axis.tag}
                    </th>
                  ))}
                  <th scope="col">{t('fontInfo.column.active')}</th>
                  <th scope="col">{t('fontInfo.column.variableInstance')}</th>
                </tr>
              </thead>
              <tbody>
                {visibleInstances.map((instance, index) => (
                  <tr key={`${instance.name}-${instance.style_name}-${index}`}>
                    <th scope="row">{instance.name}</th>
                    <td>{instance.style_name}</td>
                    <td>{instance.family_name ?? '—'}</td>
                    {axes.map((axis) => (
                      <td key={axis.tag} className={styles.numeric}>
                        {instance.location[axis.tag] ?? '—'}
                      </td>
                    ))}
                    <td>
                      {instance.active ? t('common.yes') : t('common.no')}
                    </td>
                    <td>
                      {instance.is_variable_named_instance
                        ? t('common.yes')
                        : t('common.no')}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      ) : !searching && instances.length === 0 ? (
        <>
          <h3 className={styles.subsectionTitle}>
            {t('fontInfo.group.instances')}
          </h3>
          <p className={styles.empty}>{t('fontInfo.noInstances')}</p>
        </>
      ) : null}
    </SectionBody>
  )
}
