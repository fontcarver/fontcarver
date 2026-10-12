import { afterEach, describe, expect, it } from 'vitest'

import { setTransport } from '../../../api'
import { makeCell } from '../../../test/fontFixtures'
import { createMockTransport } from '../../../test/mockTransport'
import { resolveCjkIds } from './cjkResolve'

afterEach(() => setTransport(undefined))

describe('IDS document glyph resolution', () => {
  it('resolves each unique leaf in the active master and normalizes advances', async () => {
    const transport = createMockTransport({
      'glyphs.list': () => ({
        total: 1,
        items: [makeCell({ name: 'wood', unicodes: [0x6728], width: 500 })],
      }),
    })
    setTransport(transport)
    const result = await resolveCjkIds(
      1,
      'bold',
      'forest',
      '⿰木木',
      1000,
      1000,
      -200,
    )
    expect(transport.calls).toHaveLength(1)
    expect(transport.calls[0]?.params).toMatchObject({ master: 'bold' })
    expect(result.map(({ component }) => component.transform)).toEqual([
      [1, 0, 0, 1, 0, 0],
      [1, 0, 0, 1, 500, 0],
    ])
  })

  it('uses explicit bracket names to resolve an unencoded radical', async () => {
    setTransport(
      createMockTransport({
        'glyphs.list': () => ({
          total: 2,
          items: [
            makeCell({ name: 'wood.left.alt', unicodes: [] }),
            makeCell({ name: 'wood.left', unicodes: [] }),
          ],
        }),
      }),
    )
    const result = await resolveCjkIds(
      1,
      null,
      'forest',
      '[wood.left]',
      600,
      1000,
      -200,
    )
    expect(result[0]?.component.base).toBe('wood.left')
  })

  it.each([
    ['missing', []],
    [
      'ambiguous',
      [
        makeCell({ name: 'wood', unicodes: [0x6728] }),
        makeCell({ name: 'wood.alt', unicodes: [0x6728] }),
      ],
    ],
    ['self', [makeCell({ name: 'forest', unicodes: [0x6728] })]],
    ['empty', [makeCell({ name: 'wood', unicodes: [0x6728], is_empty: true })]],
    ['width', [makeCell({ name: 'wood', unicodes: [0x6728], width: 0 })]],
  ] as const)(
    'refuses %s leaves before emitting any mutation',
    async (code, cells) => {
      const transport = createMockTransport({
        'glyphs.list': () => ({ total: cells.length, items: [...cells] }),
      })
      setTransport(transport)
      await expect(
        resolveCjkIds(1, null, 'forest', '木', 1000, 1000, -200),
      ).rejects.toMatchObject({ code })
      expect(
        transport.calls.every(({ method }) => method === 'glyphs.list'),
      ).toBe(true)
    },
  )
})
