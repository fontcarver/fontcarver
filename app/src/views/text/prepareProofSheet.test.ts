import { describe, expect, it } from 'vitest'
import { createMockTransport } from '../../test/mockTransport'
import { makeCell } from '../../test/fontFixtures'
import { prepareProofSheet } from './prepareProofSheet'

const context = {
  doc: 1,
  generation: 0,
  unitsPerEm: 1000,
  master: 'm1',
  text: '/A',
  features: [],
  direction: 'ltr',
  script: null,
  language: null,
}
const sample = {
  id: 'a',
  name: 'A proof',
  group: 'Pairs',
  text: '/A /A',
  direction: 'ltr' as const,
  script: 'Latn',
  language: 'en',
}

describe('proof sheet preparation', () => {
  it('fetches all unique outlines once and preserves every occurrence at each requested size', async () => {
    const transport = createMockTransport({
      'glyphs.search': () => [makeCell({ gid: 4, name: 'A', width: 600 })],
      'glyphs.cell': () =>
        makeCell({
          gid: 4,
          name: 'A',
          width: 600,
          path: 'M0 0L400 0L200 700Z',
        }),
    })
    const sheet = await prepareProofSheet(
      transport,
      context,
      [sample],
      [24, 48],
    )
    expect(sheet).toHaveLength(2)
    expect(sheet[0]).toMatchObject({
      label: 'A proof',
      size: 24,
      unitsPerEm: 1000,
      glyphs: [{ x: 0 }, { x: 600 }],
    })
    expect(sheet[1]?.glyphs).toEqual(sheet[0]?.glyphs)
    expect(
      transport.calls.filter((call) => call.method === 'glyphs.cell'),
    ).toHaveLength(2)
  })
  it('refuses unresolved glyphs instead of silently exporting an incomplete proof', async () => {
    const transport = createMockTransport({ 'glyphs.search': () => [] })
    await expect(
      prepareProofSheet(transport, context, [sample], [24]),
    ).rejects.toThrow('unresolved')
  })
  it('rejects changed documents before further work and bounds output size', async () => {
    const transport = createMockTransport()
    await expect(
      prepareProofSheet(transport, context, [sample], [24], {
        isCurrent: () => false,
      }),
    ).rejects.toThrow('changed')
    await expect(
      prepareProofSheet(transport, context, [sample], [0]),
    ).rejects.toThrow('size')
    expect(transport.calls).toHaveLength(0)
  })
})
