import { describe, expect, it, vi } from 'vitest'
import { setTransport, type KerningPairRecord } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import {
  DEFAULT_KERNING_FILTER,
  loadKerningTable,
  visibleKerningRecords,
  type KerningTableFilter,
} from './kerningTableData'

const filter = (
  changes: Partial<KerningTableFilter> = {},
): KerningTableFilter => ({
  ...DEFAULT_KERNING_FILTER,
  ...changes,
})

const pair = (
  left: string,
  right: string,
  value: number,
): KerningPairRecord => ({
  left: { glyph: left },
  right: { glyph: right },
  value,
  diagnostic: null,
  exception: false,
})

describe('kerning table data', () => {
  it('reads continuation pages from a single captured generation', async () => {
    const pages = vi.fn(({ offset, generation }) => {
      expect(generation).toBe(offset === 0 ? null : 7)
      return {
        status: { generation: 7, editable: true, reason: null },
        total: 201,
        items: Array.from({ length: offset === 0 ? 200 : 1 }, (_, index) =>
          pair(`A${offset + index}`, 'V', -10),
        ),
      }
    })
    setTransport(createMockTransport({ 'kerning.pairs': pages }))
    const result = await loadKerningTable(1, 'regular', false, () => true)
    expect(result?.items).toHaveLength(201)
    expect(pages).toHaveBeenCalledTimes(2)
  })

  it('does not publish or continue a cancelled read', async () => {
    let current = true
    const pages = vi.fn(() => {
      current = false
      return {
        status: { generation: 1, editable: true, reason: null },
        total: 201,
        items: [pair('A', 'V', 0)],
      }
    })
    setTransport(createMockTransport({ 'kerning.pairs_vertical': pages }))
    expect(await loadKerningTable(1, 'regular', true, () => current)).toBeNull()
    expect(pages).toHaveBeenCalledTimes(1)
  })

  it('rejects a mismatched continuation instead of mixing generations', async () => {
    setTransport(
      createMockTransport({
        'kerning.pairs': ({ offset }) => ({
          status: {
            generation: offset === 0 ? 1 : 2,
            editable: true,
            reason: null,
          },
          total: 201,
          items: Array.from({ length: offset === 0 ? 200 : 1 }, () =>
            pair('A', 'V', -20),
          ),
        }),
      }),
    )
    await expect(
      loadKerningTable(1, 'regular', false, () => true),
    ).rejects.toThrow()
  })

  it('filters group names and sorts numeric values without dropping explicit zero', () => {
    const records = [
      pair('A', 'V', -20),
      { ...pair('A', 'W', 0), left: { group: 'public.kern1.A' } },
      pair('T', 'o', 10),
    ]
    expect(
      visibleKerningRecords(records, filter(), 'value', false).map(
        (item) => item.value,
      ),
    ).toEqual([-20, 0, 10])
    expect(
      visibleKerningRecords(records, filter({ query: 'KERN1' }), 'left', true),
    ).toEqual([records[1]])
  })

  it('offers every match mode over both sides of a pair', () => {
    const records = [
      pair('A', 'V', -20),
      pair('Aacute', 'W', -10),
      pair('T', 'o', 10),
    ]
    const names = (found: readonly KerningPairRecord[]) =>
      found.map((record) => [record.left, record.right])
    expect(
      names(
        visibleKerningRecords(records, filter({ query: 'a' }), 'left', false),
      ),
    ).toEqual([
      [{ glyph: 'A' }, { glyph: 'V' }],
      [{ glyph: 'Aacute' }, { glyph: 'W' }],
    ])
    expect(
      names(
        visibleKerningRecords(
          records,
          filter({ query: 'a', match: 'is' }),
          'left',
          false,
        ),
      ),
    ).toEqual([[{ glyph: 'A' }, { glyph: 'V' }]])
    expect(
      names(
        visibleKerningRecords(
          records,
          filter({ query: 'aa', match: 'starts' }),
          'left',
          false,
        ),
      ),
    ).toEqual([[{ glyph: 'Aacute' }, { glyph: 'W' }]])
    expect(
      names(
        visibleKerningRecords(
          records,
          filter({ query: 'o', match: 'ends' }),
          'left',
          false,
        ),
      ),
    ).toEqual([[{ glyph: 'T' }, { glyph: 'o' }]])
    expect(
      names(
        visibleKerningRecords(
          records,
          filter({ query: 'a', match: 'excludes' }),
          'left',
          false,
        ),
      ),
    ).toEqual([[{ glyph: 'T' }, { glyph: 'o' }]])
  })

  it('filters by record kind, including exceptions only', () => {
    const records = [
      { ...pair('A', 'V', -20), exception: true },
      { ...pair('A', 'W', -10), left: { group: 'public.kern1.A' } },
      pair('T', 'o', 10),
    ]
    const values = (kind: KerningTableFilter['kind']) =>
      visibleKerningRecords(records, filter({ kind }), 'value', false).map(
        (record) => record.value,
      )
    expect(values('all')).toEqual([-20, -10, 10])
    expect(values('groups')).toEqual([-10])
    expect(values('glyphs')).toEqual([-20, 10])
    expect(values('exceptions')).toEqual([-20])
  })
})
