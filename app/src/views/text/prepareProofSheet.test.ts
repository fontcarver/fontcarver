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
  it('carries the text, direction, language and features of each sample', async () => {
    const transport = createMockTransport({
      'glyphs.search': () => [makeCell({ gid: 4, name: 'A', width: 600 })],
      'glyphs.cell': () =>
        makeCell({
          gid: 4,
          name: 'A',
          width: 600,
          path: 'M0 0L400 0L200 700Z',
        }),
      'text.shape': () => ({
        glyphs: [
          {
            gid: 4,
            cluster: 0,
            x_advance: 0,
            y_advance: -1000,
            x_offset: 0,
            y_offset: 0,
          },
        ],
        names: ['A'],
        units_per_em: 1000,
        size: null,
        stale: false,
        warnings: [],
      }),
    })
    const vertical = {
      ...sample,
      text: '永',
      direction: 'ttb' as const,
      script: 'Hani',
      language: 'zh-hans',
    }
    const sheet = await prepareProofSheet(
      transport,
      { ...context, features: [['kern', true] as [string, boolean]] },
      [vertical],
      [48],
    )
    expect(sheet[0]).toMatchObject({
      text: '永',
      direction: 'ttb',
      language: 'zh-hans',
      size: 48,
    })
    // The shaper is asked for the sample's own script, language and direction,
    // with the vertical features a `ttb` request implies.
    const request = transport.calls.find((call) => call.method === 'text.shape')
      ?.params as Record<string, unknown>
    expect(request).toMatchObject({
      text: '永',
      script: 'Hani',
      language: 'zh-hans',
      direction: 'ttb',
    })
    expect(JSON.stringify(request.features)).toContain('vert')
    // A vertical run keeps its negative vertical advance in the sheet.
    expect(sheet[0]?.glyphs[0]?.yAdvance).toBe(-1000)
    expect(JSON.stringify(sheet[0]?.features)).toContain('kern')
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
