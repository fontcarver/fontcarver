import { describe, expect, it } from 'vitest'

import type { PositionedGlyph } from './textLayout'
import { textClusters } from './textClusters'

function glyph(overrides: Partial<PositionedGlyph> = {}): PositionedGlyph {
  return {
    compiledGid: 1,
    documentGid: 1,
    name: 'A',
    editable: true,
    x: 0,
    y: 0,
    xAdvance: 600,
    yAdvance: 0,
    sourceByte: 0,
    sourceKind: 'literal',
    ...overrides,
  }
}

describe('shaped source clusters', () => {
  it('keeps one cluster per source offset in visual order', () => {
    const clusters = textClusters('AV', [
      glyph({ name: 'A', sourceByte: 0, x: 0 }),
      glyph({ name: 'V', sourceByte: 1, x: 600 }),
    ])
    expect(clusters.map((cluster) => cluster.index)).toEqual([0, 1])
    expect(clusters.map((cluster) => cluster.source)).toEqual(['A', 'V'])
    expect(clusters[1]!.glyphs).toEqual([
      { name: 'V', x: 600, y: 0, xAdvance: 600, yAdvance: 0 },
    ])
  })

  it('groups a base and its marks into one cluster', () => {
    const clusters = textClusters('é', [
      glyph({ name: 'e', sourceByte: 0 }),
      glyph({ name: 'acutecomb', sourceByte: 0, xAdvance: 0, y: 40 }),
    ])
    expect(clusters).toHaveLength(1)
    expect(clusters[0]!.source).toBe('é')
    expect(clusters[0]!.glyphs.map((item) => item.name)).toEqual([
      'e',
      'acutecomb',
    ])
  })

  it('maps multi-byte characters by their UTF-8 offsets', () => {
    const clusters = textClusters('永東', [
      glyph({ name: 'uni6C38', sourceByte: 0 }),
      glyph({ name: 'uni6771', sourceByte: 3 }),
    ])
    expect(clusters.map((cluster) => cluster.source)).toEqual(['永', '東'])
  })

  it('maps a right-to-left run whose offsets run backwards', () => {
    const clusters = textClusters('ab', [
      glyph({ name: 'beh', sourceByte: 1, x: 0 }),
      glyph({ name: 'alef', sourceByte: 0, x: 600 }),
    ])
    expect(clusters.map((cluster) => cluster.sourceByte)).toEqual([1, 0])
    expect(clusters.map((cluster) => cluster.source)).toEqual(['b', 'a'])
  })

  it('reports a repeated character as two clusters at its own offsets', () => {
    const clusters = textClusters('aa', [
      glyph({ sourceByte: 0 }),
      glyph({ sourceByte: 1 }),
    ])
    expect(clusters).toHaveLength(2)
    expect(clusters.every((cluster) => cluster.source === 'a')).toBe(true)
  })

  it('returns an empty source instead of throwing on an impossible offset', () => {
    expect(textClusters('a', [glyph({ sourceByte: 99 })])[0]!.source).toBe('')
    expect(textClusters('a', [glyph({ sourceByte: -1 })])[0]!.source).toBe('')
    expect(textClusters('', [])).toEqual([])
  })

  it('treats a glyph-name token as the source text it was typed as', () => {
    const clusters = textClusters('/alef x', [
      glyph({ name: 'alef', sourceByte: 0, sourceKind: 'named' }),
      glyph({ name: 'x', sourceByte: 6 }),
    ])
    // A cluster spans up to the next reported offset, so the separating space
    // belongs to the token that precedes it.
    expect(clusters.map((cluster) => cluster.source)).toEqual(['/alef ', 'x'])
  })
})
