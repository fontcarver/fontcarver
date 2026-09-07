import { describe, expect, it } from 'vitest'

import type { ShapedRun } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { cellFor } from '../../test/glyphFixtures'
import {
  buildTextLayout,
  positionShapedRun,
  runUnitsToFontUnits,
} from './textLayout'

function run(overrides: Partial<ShapedRun> = {}): ShapedRun {
  return {
    glyphs: [],
    names: [],
    warnings: [],
    stale: false,
    units_per_em: 1_000,
    size: null,
    ...overrides,
  }
}

describe('positionShapedRun', () => {
  it('keeps visual order and UTF-8 clusters for an RTL run', () => {
    const positioned = positionShapedRun(
      run({
        glyphs: [
          {
            gid: 91,
            cluster: 4,
            x_advance: 500,
            y_advance: 0,
            x_offset: 20,
            y_offset: 5,
          },
          {
            gid: 37,
            cluster: 0,
            x_advance: 600,
            y_advance: 0,
            x_offset: -10,
            y_offset: 0,
          },
        ],
        names: ['beh-ar', 'alef-ar'],
      }),
      { x: 100, y: 40 },
      12,
    )

    expect(
      positioned.glyphs.map(({ x, sourceByte }) => ({ x, sourceByte })),
    ).toEqual([
      { x: 120, sourceByte: 16 },
      { x: 590, sourceByte: 12 },
    ])
    expect(positioned.end).toEqual({ x: 1_200, y: 40 })
  })

  it('converts explicitly sized runs back to font units', () => {
    const sized = run({
      size: 20,
      units_per_em: 1_000,
      glyphs: [
        {
          gid: 1,
          cluster: 0,
          x_advance: 12,
          y_advance: 0,
          x_offset: 2,
          y_offset: 1,
        },
      ],
      names: ['A'],
    })

    expect(runUnitsToFontUnits(sized)).toBe(50)
    const positioned = positionShapedRun(sized, { x: 0, y: 0 }, 0, 'font')
    expect(positioned.glyphs[0]).toMatchObject({
      x: 100,
      y: 50,
      xAdvance: 600,
    })
    expect(positioned.end.x).toBe(600)
  })
})

describe('buildTextLayout', () => {
  it('shapes a literal span once and resolves exact names to document handles', async () => {
    const transport = createMockTransport({
      'text.shape': ({ text, size }) => {
        expect(text).toBe('ffi')
        expect(size).toBeNull()
        return run({
          glyphs: [
            {
              gid: 9_999,
              cluster: 0,
              x_advance: 600,
              y_advance: 0,
              x_offset: 0,
              y_offset: 0,
            },
          ],
          names: ['f_f_i'],
        })
      },
      'glyphs.search': ({ query }) => {
        if (query === 'f_f_i') return [cellFor(42, { name: 'f_f_i' })]
        if (query === 'unencoded')
          return [cellFor(77, { name: 'unencoded', unicodes: [], width: 620 })]
        return [cellFor(88, { name: `${query}.similar` })]
      },
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 1,
      unitsPerEm: 1_000,
      master: null,
      text: 'ffi/unencoded /missing',
      features: [['liga', true]],
      direction: 'ltr',
      script: null,
      language: null,
    })

    expect(layout.glyphs.map((glyph) => glyph.documentGid)).toEqual([
      42,
      77,
      null,
    ])
    expect(layout.glyphs[0]).toMatchObject({ compiledGid: 9_999, x: 0 })
    expect(layout.glyphs[1]).toMatchObject({ name: 'unencoded', x: 600 })
    expect(layout.end.x).toBe(1_220)
    expect(layout.diagnostics).toContainEqual(
      expect.objectContaining({ code: 'unknown-glyph-name', name: 'missing' }),
    )
    expect(
      transport.calls.filter((call) => call.method === 'text.shape'),
    ).toHaveLength(1)
  })

  it('does not guess a document handle when the compiled font has no name', async () => {
    const transport = createMockTransport({
      'text.shape': () =>
        run({
          glyphs: [
            {
              gid: 65_000,
              cluster: 0,
              x_advance: 500,
              y_advance: 0,
              x_offset: 0,
              y_offset: 0,
            },
          ],
          names: [null],
        }),
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 1,
      unitsPerEm: 2_048,
      master: null,
      text: '字',
      features: [],
      direction: null,
      script: null,
      language: null,
    })

    expect(layout.glyphs[0]).toMatchObject({
      compiledGid: 65_000,
      documentGid: null,
      editable: false,
    })
    expect(layout.diagnostics[0]?.code).toBe('missing-compiled-name')
    expect(layout.unitsPerEm).toBe(1_000)
    expect(transport.calls).toHaveLength(1)
  })

  it('maps a glyph after Unicode and // back to its original source byte', async () => {
    const transport = createMockTransport({
      'text.shape': ({ text }) => {
        expect(text).toBe('é/A')
        return run({
          glyphs: [
            {
              gid: 4,
              cluster: 3,
              x_advance: 500,
              y_advance: 0,
              x_offset: 0,
              y_offset: 0,
            },
          ],
          names: ['A'],
        })
      },
      'glyphs.search': () => [cellFor(42, { name: 'A' })],
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 1,
      unitsPerEm: 1_000,
      master: null,
      text: 'é//A',
      features: [],
      direction: null,
      script: null,
      language: null,
    })

    expect(layout.glyphs[0]?.sourceByte).toBe(4)
  })

  it('uses the selected master width and document UPEM for all-name text', async () => {
    const transport = createMockTransport({
      'glyphs.search': () => [cellFor(5, { name: 'A', width: 500 })],
      'glyphs.cell': () => cellFor(5, { name: 'A', width: 730 }),
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 4,
      unitsPerEm: 2_048,
      master: 'bold',
      text: '/A',
      features: [],
      direction: 'ltr',
      script: null,
      language: null,
    })

    expect(layout.end.x).toBe(730)
    expect(layout.unitsPerEm).toBe(2_048)
    expect(transport.calls.map((call) => call.method)).toEqual([
      'glyphs.search',
      'glyphs.cell',
    ])
  })

  it('lays out multiple RTL tokens in visual order without changing source bytes', async () => {
    const transport = createMockTransport({
      'text.shape': ({ text }) =>
        run({
          glyphs: [...text].reverse().map((_, index) => ({
            gid: 100 + index,
            cluster: text.length - index - 1,
            x_advance: 100,
            y_advance: 0,
            x_offset: 0,
            y_offset: 0,
          })),
          names: [...text].reverse(),
        }),
      'glyphs.search': ({ query }) => [
        cellFor(query.charCodeAt(0), { name: query }),
      ],
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 1,
      unitsPerEm: 1_000,
      master: null,
      text: 'ab/X cd',
      features: [],
      direction: 'rtl',
      script: null,
      language: null,
    })

    expect(layout.glyphs.map((glyph) => glyph.name)).toEqual([
      'd',
      'c',
      'X',
      'b',
      'a',
    ])
    expect(layout.glyphs.map((glyph) => glyph.sourceByte)).toEqual([
      6, 5, 2, 1, 0,
    ])
  })

  it('reverses an all-name RTL run', async () => {
    const transport = createMockTransport({
      'glyphs.search': ({ query }) => [
        cellFor(query === 'A' ? 1 : 2, { name: query, width: 500 }),
      ],
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 1,
      unitsPerEm: 1_000,
      master: null,
      text: '/A /B',
      features: [],
      direction: 'rtl',
      script: null,
      language: null,
    })

    expect(layout.glyphs.map((glyph) => glyph.name)).toEqual(['B', 'A'])
  })

  it('bounds exact-name search concurrency for ordinary runs', async () => {
    let active = 0
    let peak = 0
    const transport = createMockTransport({
      'glyphs.search': async ({ query }) => {
        active += 1
        peak = Math.max(peak, active)
        await new Promise((resolve) => setTimeout(resolve, 1))
        active -= 1
        return [cellFor(Number(query.slice(1)), { name: query })]
      },
    })
    const names = Array.from({ length: 20 }, (_, index) => `/g${index}`).join(
      ' ',
    )

    await buildTextLayout(
      transport,
      {
        doc: 1,
        generation: 1,
        unitsPerEm: 1_000,
        master: null,
        text: names,
        features: [],
        direction: 'ltr',
        script: null,
        language: null,
      },
      { searchConcurrency: 3, listThreshold: 100 },
    )

    expect(peak).toBe(3)
  })

  it('stops queued exact-name searches after the first lookup failure', async () => {
    const searched: string[] = []
    const transport = createMockTransport({
      'glyphs.search': async ({ query }) => {
        searched.push(query)
        if (query === 'g0') {
          await new Promise((resolve) => setTimeout(resolve, 5))
          throw new Error('lookup failed')
        }
        await new Promise((resolve) => setTimeout(resolve, 15))
        return [cellFor(1, { name: query })]
      },
    })

    await expect(
      buildTextLayout(
        transport,
        {
          doc: 1,
          generation: 1,
          unitsPerEm: 1_000,
          master: null,
          text: '/g0 /g1 /g2 /g3',
          features: [],
          direction: 'ltr',
          script: null,
          language: null,
        },
        { searchConcurrency: 2, listThreshold: 100 },
      ),
    ).rejects.toThrow('lookup failed')
    expect(searched).toEqual(['g0', 'g1'])
  })

  it('uses paged glyph listing instead of per-name searches for large runs', async () => {
    const names = Array.from({ length: 70 }, (_, index) => `g${index}`)
    const transport = createMockTransport({
      'glyphs.list': ({ offset, limit }) => ({
        total: names.length,
        items: names
          .slice(offset, offset + limit)
          .map((name, index) => cellFor(offset + index, { name })),
      }),
    })

    const layout = await buildTextLayout(transport, {
      doc: 1,
      generation: 1,
      unitsPerEm: 1_000,
      master: null,
      text: names.map((name) => `/${name}`).join(' '),
      features: [],
      direction: 'ltr',
      script: null,
      language: null,
    })

    expect(layout.glyphs.every((glyph) => glyph.editable)).toBe(true)
    expect(transport.calls.map((call) => call.method)).toEqual(['glyphs.list'])
  })
})
