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

/** Case-insensitive pair-name filtering and stable numeric/name sorting. */
export function visibleKerningRecords(
  records: readonly KerningPairRecord[],
  query: string,
  sort: KerningTableSort,
  descending: boolean,
): KerningPairRecord[] {
  const needle = query.trim().toLocaleLowerCase()
  return records
    .filter(
      (record) =>
        !needle ||
        [record.left, record.right].some((side) =>
          kerningSideName(side).toLocaleLowerCase().includes(needle),
        ),
    )
    .sort((a, b) => {
      const order =
        sort === 'value'
          ? (a.value ?? Number.POSITIVE_INFINITY) -
            (b.value ?? Number.POSITIVE_INFINITY)
          : kerningSideName(a[sort]).localeCompare(kerningSideName(b[sort]))
      return descending ? -order : order
    })
}
