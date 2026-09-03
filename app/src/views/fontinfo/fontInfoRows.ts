/**
 * The read-only field model behind the Font Info tab.
 *
 * Building label/value rows as data rather than JSX keeps the sections short,
 * makes the cross-section search a filter over one list, and lets the whole
 * mapping — including the awkward OpenType corners — be unit tested without
 * rendering anything.
 */

import type { DocSummary, FontInfo } from '../../api'

/** One labelled, read-only value. */
export interface InfoRow {
  label: string
  value: string
}

/** A titled group of rows inside a section. */
export interface InfoGroup {
  id: string
  title: string
  rows: InfoRow[]
}

/** The translator the builders use; tests pass the identity function. */
export type Translate = (key: string) => string

const DASH = '—'

function text(value: string | null | undefined): string {
  const trimmed = value?.trim()
  return trimmed ? trimmed : DASH
}

function num(value: number | null | undefined): string {
  return value === null || value === undefined ? DASH : String(value)
}

function bool(value: boolean | null | undefined, t: Translate): string {
  if (value === null || value === undefined) return DASH
  return value ? t('common.yes') : t('common.no')
}

/** Seconds between the OpenType `LONGDATETIME` epoch and the Unix epoch. */
const MAC_EPOCH_OFFSET = 2_082_844_800

/**
 * `head.created` reaches the UI either already formatted or as the raw
 * `LONGDATETIME` count of seconds since 1904-01-01, depending on the source
 * format. A bare number is rendered as an ISO date so both read alike.
 */
export function formatTimestamp(value: string | null): string {
  const raw = value?.trim()
  if (!raw) return DASH
  if (!/^-?\d+$/.test(raw)) return raw
  const seconds = Number(raw) - MAC_EPOCH_OFFSET
  const date = new Date(seconds * 1000)
  if (Number.isNaN(date.getTime())) return raw
  return date
    .toISOString()
    .replace('T', ' ')
    .replace(/\.\d+Z$/, ' UTC')
}

function bits(values: readonly number[]): string {
  return values.length === 0 ? DASH : values.join(', ')
}

function tuple(values: readonly number[] | null): string {
  return values === null ? DASH : values.join(' ')
}

function group(id: string, t: Translate, rows: [string, string][]): InfoGroup {
  return {
    id,
    title: t(`fontInfo.group.${id}`),
    rows: rows.map(([key, value]) => ({ label: t(key), value })),
  }
}

/** Rows of the **Font** section: identity, version, authorship and legal. */
export function fontGroups(
  info: FontInfo,
  summary: DocSummary,
  t: Translate,
): InfoGroup[] {
  const groups: InfoGroup[] = [
    group('identification', t, [
      ['fontInfo.field.familyName', text(info.family_name)],
      ['fontInfo.field.styleName', text(info.style_name)],
      [
        'fontInfo.field.preferredFamily',
        text(info.open_type_name_preferred_family_name),
      ],
      [
        'fontInfo.field.preferredSubfamily',
        text(info.open_type_name_preferred_subfamily_name),
      ],
      ['fontInfo.field.wwsFamily', text(info.open_type_name_wws_family_name)],
      [
        'fontInfo.field.wwsSubfamily',
        text(info.open_type_name_wws_subfamily_name),
      ],
      [
        'fontInfo.field.compatibleFull',
        text(info.open_type_name_compatible_full_name),
      ],
      ['fontInfo.field.postscriptName', text(info.postscript_font_name)],
      ['fontInfo.field.postscriptFullName', text(info.postscript_full_name)],
      ['fontInfo.field.uniqueId', text(info.open_type_name_unique_id)],
      ['fontInfo.field.description', text(info.open_type_name_description)],
      ['fontInfo.field.sampleText', text(info.open_type_name_sample_text)],
    ]),
    group('version', t, [
      [
        'fontInfo.field.version',
        `${info.version_major}.${String(info.version_minor).padStart(3, '0')}`,
      ],
      ['fontInfo.field.versionString', text(info.open_type_name_version)],
      ['fontInfo.field.created', formatTimestamp(info.open_type_head_created)],
      ['fontInfo.field.unitsPerEm', num(info.units_per_em)],
      ['fontInfo.field.format', text(summary.format)],
      ['fontInfo.field.glyphCount', summary.glyph_count.toLocaleString()],
      ['fontInfo.field.path', text(summary.path)],
    ]),
    group('authorship', t, [
      ['fontInfo.field.designer', text(info.open_type_name_designer)],
      ['fontInfo.field.designerUrl', text(info.open_type_name_designer_url)],
      ['fontInfo.field.manufacturer', text(info.open_type_name_manufacturer)],
      [
        'fontInfo.field.manufacturerUrl',
        text(info.open_type_name_manufacturer_url),
      ],
      ['fontInfo.field.vendorId', text(info.open_type_os2_vendor_id)],
    ]),
    group('legal', t, [
      ['fontInfo.field.copyright', text(info.copyright)],
      ['fontInfo.field.trademark', text(info.trademark)],
      ['fontInfo.field.license', text(info.open_type_name_license)],
      ['fontInfo.field.licenseUrl', text(info.open_type_name_license_url)],
    ]),
    group('note', t, [['fontInfo.field.note', text(info.note)]]),
  ]

  if (summary.warnings.length > 0)
    groups.push({
      id: 'warnings',
      title: t('fontInfo.group.warnings'),
      rows: summary.warnings.map((warning, index) => ({
        label: `${index + 1}`,
        value: warning,
      })),
    })

  return groups
}

/** Rows of the **Other** section: the OpenType tables the model carries. */
export function otherGroups(info: FontInfo, t: Translate): InfoGroup[] {
  return [
    group('os2', t, [
      ['fontInfo.field.weightClass', num(info.open_type_os2_weight_class)],
      ['fontInfo.field.widthClass', num(info.open_type_os2_width_class)],
      ['fontInfo.field.selection', bits(info.open_type_os2_selection)],
      ['fontInfo.field.embedding', bits(info.open_type_os2_type)],
      ['fontInfo.field.familyClass', tuple(info.open_type_os2_family_class)],
      ['fontInfo.field.panose', tuple(info.open_type_os2_panose)],
      ['fontInfo.field.unicodeRanges', bits(info.open_type_os2_unicode_ranges)],
      [
        'fontInfo.field.codePageRanges',
        bits(info.open_type_os2_code_page_ranges),
      ],
      [
        'fontInfo.field.subscript',
        `${num(info.open_type_os2_subscript_x_size)} × ${num(
          info.open_type_os2_subscript_y_size,
        )} @ ${num(info.open_type_os2_subscript_x_offset)}, ${num(
          info.open_type_os2_subscript_y_offset,
        )}`,
      ],
      [
        'fontInfo.field.superscript',
        `${num(info.open_type_os2_superscript_x_size)} × ${num(
          info.open_type_os2_superscript_y_size,
        )} @ ${num(info.open_type_os2_superscript_x_offset)}, ${num(
          info.open_type_os2_superscript_y_offset,
        )}`,
      ],
      [
        'fontInfo.field.strikeout',
        `${num(info.open_type_os2_strikeout_size)} @ ${num(
          info.open_type_os2_strikeout_position,
        )}`,
      ],
    ]),
    group('head', t, [
      ['fontInfo.field.headFlags', bits(info.open_type_head_flags)],
      [
        'fontInfo.field.lowestRecPpem',
        num(info.open_type_head_lowest_rec_ppem),
      ],
      [
        'fontInfo.field.gasp',
        info.open_type_gasp_range_records.length === 0
          ? DASH
          : info.open_type_gasp_range_records
              .map(
                (record) =>
                  `≤${record.range_max_ppem}: ${
                    record.behavior.join(',') || '0'
                  }`,
              )
              .join('  '),
      ],
    ]),
    group('caret', t, [
      [
        'fontInfo.field.hheaCaret',
        `${num(info.open_type_hhea_caret_slope_rise)} / ${num(
          info.open_type_hhea_caret_slope_run,
        )} @ ${num(info.open_type_hhea_caret_offset)}`,
      ],
      [
        'fontInfo.field.vheaCaret',
        `${num(info.open_type_vhea_caret_slope_rise)} / ${num(
          info.open_type_vhea_caret_slope_run,
        )} @ ${num(info.open_type_vhea_caret_offset)}`,
      ],
    ]),
    group('postscript', t, [
      ['fontInfo.field.isFixedPitch', bool(info.postscript_is_fixed_pitch, t)],
      ['fontInfo.field.defaultWidthX', num(info.postscript_default_width_x)],
      ['fontInfo.field.nominalWidthX', num(info.postscript_nominal_width_x)],
      [
        'fontInfo.field.underline',
        `${num(info.postscript_underline_thickness)} @ ${num(
          info.postscript_underline_position,
        )}`,
      ],
      ['fontInfo.field.psUniqueId', num(info.postscript_unique_id)],
    ]),
    group('macintosh', t, [
      ['fontInfo.field.fondName', text(info.macintosh_fond_name)],
      ['fontInfo.field.fondFamilyId', num(info.macintosh_fond_family_id)],
    ]),
  ]
}

/** Whether a row matches a search term; the search is case-insensitive. */
export function rowMatches(row: InfoRow, query: string): boolean {
  if (query === '') return true
  const needle = query.toLowerCase()
  return (
    row.label.toLowerCase().includes(needle) ||
    row.value.toLowerCase().includes(needle)
  )
}

/** Drops rows, and then empty groups, that do not match a search term. */
export function filterGroups(
  groups: readonly InfoGroup[],
  query: string,
): InfoGroup[] {
  if (query.trim() === '') return [...groups]
  const needle = query.trim()
  return groups
    .map((group) => ({
      ...group,
      rows: group.rows.filter((row) => rowMatches(row, needle)),
    }))
    .filter((group) => group.rows.length > 0)
}
