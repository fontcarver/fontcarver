import { useTranslation } from 'react-i18next'

import type { Master } from '../../api'

import styles from './fontInfo.module.css'
import { rowMatches } from './fontInfoRows'
import { SectionBody } from './SectionBody'
import {
  resolveMasterMetric,
  type OptionalMasterMetric,
} from './metricInheritance'

const DASH = '—'

function num(value: number | null | undefined): string {
  return value === null || value === undefined ? DASH : String(value)
}

function location(master: Master): string {
  const entries = Object.entries(master.location)
  if (entries.length === 0) return DASH
  return entries.map(([axis, value]) => `${axis} ${value}`).join('  ')
}

function zones(master: Master): string {
  if (master.alignment_zones.length === 0) return DASH
  return master.alignment_zones
    .map((zone) => `${zone.position}/${zone.size}`)
    .join('  ')
}

function stems(master: Master): string {
  if (master.stems.length === 0) return DASH
  return master.stems
    .map(
      (stem) => `${stem.name || (stem.horizontal ? 'h' : 'v')} ${stem.width}`,
    )
    .join('  ')
}

export interface MetricsTableProps {
  masters: readonly Master[]
  unitsPerEm: number
  /** Search term applied to the row label and every cell. */
  query: string
  heading?: string
}

/**
 * One column per master, one row per metric, which is how a type designer
 * compares masters: reading across a row shows immediately whether the
 * vertical metrics are consistent.
 */
export function MetricsTable({
  masters,
  unitsPerEm,
  query,
  heading,
}: MetricsTableProps) {
  const { t } = useTranslation()

  const inherited = (field: OptionalMasterMetric) => (master: Master) =>
    num(resolveMasterMetric(master.metrics, field, unitsPerEm).value)
  const rows: [string, (master: Master) => string, OptionalMasterMetric?][] = [
    ['fontInfo.metric.location', location],
    ['fontInfo.metric.ascender', (m) => num(m.metrics.ascender)],
    ['fontInfo.metric.descender', (m) => num(m.metrics.descender)],
    ['fontInfo.metric.xHeight', (m) => num(m.metrics.x_height)],
    ['fontInfo.metric.capHeight', (m) => num(m.metrics.cap_height)],
    ['fontInfo.metric.italicAngle', (m) => num(m.metrics.italic_angle)],
    [
      'fontInfo.metric.hheaAscender',
      inherited('hhea_ascender'),
      'hhea_ascender',
    ],
    [
      'fontInfo.metric.hheaDescender',
      inherited('hhea_descender'),
      'hhea_descender',
    ],
    [
      'fontInfo.metric.hheaLineGap',
      inherited('hhea_line_gap'),
      'hhea_line_gap',
    ],
    [
      'fontInfo.metric.typoAscender',
      inherited('typo_ascender'),
      'typo_ascender',
    ],
    [
      'fontInfo.metric.typoDescender',
      inherited('typo_descender'),
      'typo_descender',
    ],
    [
      'fontInfo.metric.typoLineGap',
      inherited('typo_line_gap'),
      'typo_line_gap',
    ],
    ['fontInfo.metric.winAscent', inherited('win_ascent'), 'win_ascent'],
    ['fontInfo.metric.winDescent', inherited('win_descent'), 'win_descent'],
    [
      'fontInfo.metric.vheaAscender',
      inherited('vhea_ascender'),
      'vhea_ascender',
    ],
    [
      'fontInfo.metric.vheaDescender',
      inherited('vhea_descender'),
      'vhea_descender',
    ],
    [
      'fontInfo.metric.vheaLineGap',
      inherited('vhea_line_gap'),
      'vhea_line_gap',
    ],
    [
      'fontInfo.metric.verticalOrigin',
      inherited('default_vertical_origin'),
      'default_vertical_origin',
    ],
    [
      'fontInfo.metric.verticalAdvance',
      inherited('default_vertical_advance'),
      'default_vertical_advance',
    ],
    ['fontInfo.metric.zones', zones],
    ['fontInfo.metric.stems', stems],
    ['fontInfo.metric.guides', (m) => String(m.guides.length)],
  ]

  const visible =
    masters.length === 0
      ? []
      : rows.filter(([key, read]) =>
          rowMatches(
            { label: t(key), value: masters.map(read).join(' ') },
            query,
          ),
        )

  return (
    <SectionBody
      heading={heading}
      hasContent={visible.length > 0}
      empty={
        masters.length === 0 ? t('fontInfo.noMasters') : t('fontInfo.noMatches')
      }
    >
      <div className={styles.tableScroll}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th scope="col">{t('fontInfo.column.metric')}</th>
              {masters.map((master) => (
                <th key={master.id} scope="col">
                  {master.name}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {visible.map(([key, read, field]) => (
              <tr key={key}>
                <th scope="row">{t(key)}</th>
                {masters.map((master) => {
                  const resolved = field
                    ? resolveMasterMetric(master.metrics, field, unitsPerEm)
                    : null
                  return (
                    <td key={master.id} className={styles.numeric}>
                      {read(master)}
                      {resolved && (
                        <span className={styles.sourceLabel}>
                          {t(
                            resolved.overridden
                              ? 'scopedInfo.masterOverride'
                              : `scopedInfo.metricSource.${resolved.source}`,
                          )}
                        </span>
                      )}
                    </td>
                  )
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SectionBody>
  )
}
