import {
  call,
  type DocId,
  type KerningPairRecord,
  type KerningPairs,
} from '../../api'
import { ApiCallError } from '../../transport'
import { kerningSideName } from './kerningModel'

/** Columns sorted across the complete table before visual pagination. */
export type KerningTableSort = 'left' | 'right' | 'value'

/** How the filter text is compared with a pair side's name. */
export type KerningMatchMode =
  'contains' | 'is' | 'starts' | 'ends' | 'excludes'

/** Which kind of record the table shows. */
export type KerningRecordKind = 'all' | 'groups' | 'glyphs' | 'exceptions'

/** Everything the table filters on, independent of sorting and paging. */
export interface KerningTableFilter {
  query: string
  match: KerningMatchMode
  kind: KerningRecordKind
}

/** The filter a freshly opened table uses. */
export const DEFAULT_KERNING_FILTER: KerningTableFilter = {
  query: '',
  match: 'contains',
  kind: 'all',
}

/** Read bounded pages at one generation, abandoning superseded requests. */
export async function loadKerningTable(
  doc: DocId,
  master: string,
  vertical: boolean,
  current: () => boolean,
): Promise<KerningPairs | null> {
  let result: KerningPairs | null = null
  while (current()) {
    const page: KerningPairs = await call(
      vertical ? 'kerning.pairs_vertical' : 'kerning.pairs',
      {
        doc,
        master,
        generation: result?.status.generation ?? null,
        offset: result?.items.length ?? 0,
        limit: 200,
      },
    )
    if (!current()) return null
    if (
      result &&
      (page.status.generation !== result.status.generation ||
        page.total !== result.total)
    )
      throw new ApiCallError(
        'stale_generation',
        'Kerning table changed while loading',
      )
    if (!result) result = { ...page, items: [...page.items] }
    else result.items.push(...page.items)
    if (result.items.length >= result.total) return result
    if (page.items.length === 0)
      throw new ApiCallError(
        'invalid_response',
        'Kerning pagination made no progress',
      )
  }
  return null
}

function matches(
  name: string,
  needle: string,
  mode: KerningMatchMode,
): boolean {
  switch (mode) {
    case 'is':
      return name === needle
    case 'starts':
      return name.startsWith(needle)
    case 'ends':
      return name.endsWith(needle)
    case 'excludes':
      return !name.includes(needle)
    default:
      return name.includes(needle)
  }
}

function keepsKind(
  record: KerningPairRecord,
  kind: KerningRecordKind,
): boolean {
  switch (kind) {
    case 'groups':
      return 'group' in record.left || 'group' in record.right
    case 'glyphs':
      return 'glyph' in record.left && 'glyph' in record.right
    case 'exceptions':
      return record.exception === true
    default:
      return true
  }
}

/**
 * Filters by side name and record kind, then sorts the whole table.
 *
 * Sorting precedes pagination so a page is a window on the ordered table rather
 * than a sorted window. "Excludes" is the negative match mode, so a name is
 * compared once and the whole record is kept when *either* side matches — the
 * useful reading for "show me everything about @O" — while "excludes" keeps a
 * record only when neither side contains the text.
 */
export function visibleKerningRecords(
  records: readonly KerningPairRecord[],
  filter: KerningTableFilter,
  sort: KerningTableSort,
  descending: boolean,
): KerningPairRecord[] {
  const needle = filter.query.trim().toLocaleLowerCase()
  return records
    .filter((record) => {
      if (!keepsKind(record, filter.kind)) return false
      if (!needle) return true
      const names = [record.left, record.right].map((side) =>
        kerningSideName(side).toLocaleLowerCase(),
      )
      return filter.match === 'excludes'
        ? names.every((name) => matches(name, needle, 'excludes'))
        : names.some((name) => matches(name, needle, filter.match))
    })
    .sort((a, b) => {
      const order =
        sort === 'value'
          ? (a.value ?? Number.POSITIVE_INFINITY) -
            (b.value ?? Number.POSITIVE_INFINITY)
          : kerningSideName(a[sort]).localeCompare(kerningSideName(b[sort]))
      return descending ? -order : order
    })
}
