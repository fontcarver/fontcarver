import { describe, expect, it } from 'vitest'
import { glyphText, reconcileTextIndex, textTitle } from './textState'
import type { PositionedGlyph } from './textLayout'

const glyph = (sourceByte: number): PositionedGlyph => ({
  sourceByte,
  name: 'a',
  documentGid: 8,
  compiledGid: 2,
  editable: true,
  x: 0,
  y: 0,
  xAdvance: 600,
  yAdvance: 0,
  sourceKind: 'literal',
})
describe('text tab identity', () => {
  it('titles Unicode characters and initializes unencoded names', () => {
    expect(textTitle('😀'.repeat(30))).toBe('😀'.repeat(24))
    expect(glyphText({ name: 'a.alt', unicodes: [] })).toBe('/a.alt')
    expect(glyphText({ name: 'smile', unicodes: [0x1f600] })).toBe('😀')
  })
  it('retains the source occurrence when repeated glyphs reorder visually', () => {
    expect(reconcileTextIndex(glyph(1), [glyph(2), glyph(1), glyph(0)])).toBe(1)
    expect(reconcileTextIndex(glyph(9), [glyph(2), glyph(1)])).toBeNull()
  })
})
