import { useTranslation } from 'react-i18next'

import type { Master } from '../../api'

import styles from './fontInfo.module.css'
import { rowMatches } from './fontInfoRows'
import { SectionBody } from './SectionBody'

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
  /** Search term applied to the row label and every cell. */
  query: string
  heading?: string
}

/**
 * One column per master, one row per metric, which is how a type designer
 * compares masters: reading across a row shows immediately whether the
 * vertical metrics are consistent.
 */
export function MetricsTable({ masters, query, heading }: MetricsTableProps) {
  const { t } = useTranslation()

  const rows: [string, (master: Master) => string][] = [
    ['fontInfo.metric.location', location],
    ['fontInfo.metric.ascender', (m) => num(m.metrics.ascender)],
    ['fontInfo.metric.descender', (m) => num(m.metrics.descender)],
    ['fontInfo.metric.xHeight', (m) => num(m.metrics.x_height)],
    ['fontInfo.metric.capHeight', (m) => num(m.metrics.cap_height)],
    ['fontInfo.metric.italicAngle', (m) => num(m.metrics.italic_angle)],
    ['fontInfo.metric.hheaAscender', (m) => num(m.metrics.hhea_ascender)],
    ['fontInfo.metric.hheaDescender', (m) => num(m.metrics.hhea_descender)],
    ['fontInfo.metric.hheaLineGap', (m) => num(m.metrics.hhea_line_gap)],
    ['fontInfo.metric.typoAscender', (m) => num(m.metrics.typo_ascender)],
    ['fontInfo.metric.typoDescender', (m) => num(m.metrics.typo_descender)],
    ['fontInfo.metric.typoLineGap', (m) => num(m.metrics.typo_line_gap)],
    ['fontInfo.metric.winAscent', (m) => num(m.metrics.win_ascent)],
    ['fontInfo.metric.winDescent', (m) => num(m.metrics.win_descent)],
    ['fontInfo.metric.vheaAscender', (m) => num(m.metrics.vhea_ascender)],
    ['fontInfo.metric.vheaDescender', (m) => num(m.metrics.vhea_descender)],
    ['fontInfo.metric.vheaLineGap', (m) => num(m.metrics.vhea_line_gap)],
    [
      'fontInfo.metric.verticalOrigin',
      (m) => num(m.metrics.default_vertical_origin),
    ],
    [
      'fontInfo.metric.verticalAdvance',
      (m) => num(m.metrics.default_vertical_advance),
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
                <th key={master.id.toString()} scope="col">
                  {master.name}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {visible.map(([key, read]) => (
              <tr key={key}>
                <th scope="row">{t(key)}</th>
                {masters.map((master) => (
                  <td key={master.id.toString()} className={styles.numeric}>
                    {read(master)}
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
