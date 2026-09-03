import { describe, expect, it } from 'vitest'

import { makeFontInfo } from '../../test/fontFixtures'
import { makeSummary } from '../../test/mockTransport'

import {
  filterGroups,
  fontGroups,
  formatTimestamp,
  otherGroups,
} from './fontInfoRows'

/** Identity translator so the assertions read the i18n keys directly. */
const t = (key: string) => key

function rowValue(
  groups: ReturnType<typeof fontGroups>,
  label: string,
): string | undefined {
  for (const group of groups)
    for (const row of group.rows) if (row.label === label) return row.value
  return undefined
}

describe('fontGroups', () => {
  const groups = fontGroups(makeFontInfo(), makeSummary(), t)

  it('renders identity, version, authorship and legal groups', () => {
    expect(groups.map((group) => group.id)).toEqual([
      'identification',
      'version',
      'authorship',
      'legal',
      'note',
    ])
  })

  it('formats the version as major.minor with three minor digits', () => {
    expect(rowValue(groups, 'fontInfo.field.version')).toBe('2.037')
  })

  it('shows an em dash for a missing value', () => {
    expect(rowValue(groups, 'fontInfo.field.trademark')).toBe('—')
    expect(rowValue(groups, 'fontInfo.field.designer')).toBe('—')
  })

  it('takes the format, glyph count and path from the document summary', () => {
    expect(rowValue(groups, 'fontInfo.field.format')).toBe('TrueType')
    expect(rowValue(groups, 'fontInfo.field.glyphCount')).toBe(
      (6253).toLocaleString(),
    )
    expect(rowValue(groups, 'fontInfo.field.path')).toBe(
      '/fonts/DejaVuSans.ttf',
    )
  })

  it('appends the import warnings when the document has any', () => {
    const withWarnings = fontGroups(
      makeFontInfo(),
      makeSummary({
        warnings: [
          { code: 'post-table-version', message: 'post table version 3' },
        ],
      }),
      t,
    )
    const warnings = withWarnings.at(-1)
    expect(warnings?.id).toBe('warnings')
    expect(warnings?.rows).toEqual([
      { label: '1', value: 'post table version 3' },
    ])
  })
})

describe('otherGroups', () => {
  const groups = otherGroups(
    makeFontInfo({
      open_type_os2_panose: [2, 11, 6, 3, 3, 8, 4, 2, 2, 4],
      open_type_gasp_range_records: [
        { range_max_ppem: 65535, behavior: [0, 1] },
      ],
      open_type_os2_type: [2],
    }),
    t,
  )

  it('lists the OpenType tables the model carries', () => {
    expect(groups.map((group) => group.id)).toEqual([
      'os2',
      'head',
      'caret',
      'postscript',
      'macintosh',
    ])
  })

  it('renders bit lists, tuples and gasp ranges readably', () => {
    expect(rowValue(groups, 'fontInfo.field.panose')).toBe(
      '2 11 6 3 3 8 4 2 2 4',
    )
    expect(rowValue(groups, 'fontInfo.field.embedding')).toBe('2')
    expect(rowValue(groups, 'fontInfo.field.unicodeRanges')).toBe('—')
    expect(rowValue(groups, 'fontInfo.field.gasp')).toBe('≤65535: 0,1')
  })

  it('combines paired values into one row', () => {
    expect(rowValue(groups, 'fontInfo.field.underline')).toBe('90 @ -130')
    expect(rowValue(groups, 'fontInfo.field.isFixedPitch')).toBe('common.no')
  })
})

describe('formatTimestamp', () => {
  it('renders a raw LONGDATETIME count as a date', () => {
    // 3,833,075,251 seconds after 1904-01-01 is 2025-06-18.
    expect(formatTimestamp('3833075251')).toBe('2025-06-18 07:07:31 UTC')
  })

  it('passes an already formatted timestamp through', () => {
    expect(formatTimestamp('2010/01/01 00:00:00')).toBe('2010/01/01 00:00:00')
    expect(formatTimestamp(null)).toBe('—')
    expect(formatTimestamp('   ')).toBe('—')
  })
})

describe('filterGroups', () => {
  const groups = fontGroups(makeFontInfo(), makeSummary(), t)

  it('returns everything for an empty search', () => {
    expect(filterGroups(groups, '   ')).toHaveLength(groups.length)
  })

  it('keeps only matching rows and drops emptied groups', () => {
    const filtered = filterGroups(groups, 'copyright')
    expect(filtered).toHaveLength(1)
    expect(filtered[0]?.id).toBe('legal')
    expect(filtered[0]?.rows).toHaveLength(1)
  })

  it('matches on the value as well as the label', () => {
    const filtered = filterGroups(groups, 'bitstream')
    expect(filtered.flatMap((group) => group.rows)).toHaveLength(1)
  })
})
