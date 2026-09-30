import { describe, expect, it } from 'vitest'
import {
  defaultVisualLookup,
  isValidFeatureTag,
  MAX_VISUAL_GLYPHS,
  parseVisualLookup,
  serializeVisualLookup,
  validateVisualLookup,
  VISUAL_LOOKUP_KINDS,
  type VisualLookup,
} from './visualLookup'

const zero = { xPlacement: 0, yPlacement: 0, xAdvance: 0, yAdvance: 0 }
const anchor = { x: 100, y: -200 }
const rules: VisualLookup[] = [
  { kind: 'gsub-single', from: 'a b', to: 'a.alt b.alt' },
  { kind: 'gsub-multiple', from: 'a', to: 'a b' },
  { kind: 'gsub-alternate', from: 'a', to: 'a.alt a.ss01' },
  { kind: 'gsub-ligature', from: 'f i', to: 'f_i' },
  {
    kind: 'gsub-contextual',
    before: 'a b',
    input: 'c',
    after: 'd e',
    replacement: 'c.alt',
  },
  { kind: 'gpos-single', glyphs: 'A V', value: { ...zero, yPlacement: 20 } },
  {
    kind: 'gpos-pair',
    first: 'A',
    second: 'V W',
    firstValue: { ...zero, xAdvance: -80 },
    secondValue: { ...zero, xPlacement: 10 },
  },
  ...(['gpos-mark-base', 'gpos-mark-mark'] as const).map((kind) => ({
    kind,
    markGlyphs: 'acute grave',
    markClass: 'visual_mark',
    markAnchor: anchor,
    targetGlyphs: kind === 'gpos-mark-base' ? 'A V' : 'tilde',
    targetAnchor: { x: 200, y: 700 },
  })),
]

describe('visual lookup FEA', () => {
  it.each(rules)('round trips $kind', (rule) => {
    expect(validateVisualLookup(rule)).toEqual([])
    const code = serializeVisualLookup(rule)
    expect(parseVisualLookup(code)).toEqual(rule)
    expect(
      parseVisualLookup(`# before\n${code.replaceAll(';', '; # after\r\n')}`),
    ).toEqual(rule)
  })

  it('renders every default without throwing while exposing incomplete fields', () => {
    for (const kind of VISUAL_LOOKUP_KINDS) {
      const rule = defaultVisualLookup(kind)
      expect(rule.kind).toBe(kind)
      expect(() => serializeVisualLookup(rule)).not.toThrow()
      expect(validateVisualLookup(rule).length).toBeGreaterThan(0)
      expect(parseVisualLookup(serializeVisualLookup(rule))).toBeNull()
    }
  })

  it('understands statement aliases without losing positioning semantics', () => {
    expect(parseVisualLookup('position A <0 0 50 0>;')).toEqual({
      kind: 'gpos-single',
      glyphs: 'A',
      value: { ...zero, xAdvance: 50 },
    })
    expect(parseVisualLookup('substitute a by b;')).toEqual({
      kind: 'gsub-single',
      from: 'a',
      to: 'b',
    })
  })

  it('escapes glyph names which are keywords or contain hyphens', () => {
    const rule: VisualLookup = {
      kind: 'gsub-single',
      from: 'sub A-Z .notdef',
      to: 'by pos NULL',
    }
    expect(serializeVisualLookup(rule)).toBe(
      'sub [\\sub \\A-Z \\.notdef] by [\\by \\pos \\NULL];',
    )
    expect(parseVisualLookup(serializeVisualLookup(rule))).toEqual(rule)
    expect(parseVisualLookup('sub [A-Z] by a;')).toBeNull()
  })

  it('accepts one marked input with context on either side', () => {
    expect(parseVisualLookup("sub a' b by c;")).toEqual({
      kind: 'gsub-contextual',
      before: '',
      input: 'a',
      after: 'b',
      replacement: 'c',
    })
    expect(parseVisualLookup("sub a b' by c;")).toEqual({
      kind: 'gsub-contextual',
      before: 'a',
      input: 'b',
      after: '',
      replacement: 'c',
    })
    expect(parseVisualLookup("sub a' by c;")?.kind).toBe('gsub-contextual')
  })

  it.each([
    '',
    'sub a by b; sub c by d;',
    'lookup L { sub a by b; } L;',
    'lookupflag IgnoreMarks; sub a by b;',
    'sub @letters by @alternates;',
    'sub a by NULL;',
    'sub [a b] by [c];',
    'sub a by b c d; garbage',
    "sub a' b' by c;",
    "sub a' lookup L b;",
    "sub [a b]' by c;",
    'ignore sub a;',
    'rsub a by b;',
    'pos cursive a <anchor 0 0> <anchor 0 0>;',
    'pos A <0 0 -80 0 <device NULL> <device NULL> <device NULL> <device NULL>>;',
    'pos A <0 0 -80 0>; # ok\ninclude(extra.fea);',
    'markClass acute <anchor 0 0> @top; pos base A <anchor 1 2> mark @other;',
    'markClass acute <anchor 0 0> @top; pos base A <anchor NULL> mark @top;',
    'markClass acute <anchor 0 0> @top; pos base A <anchor 0 0 contourpoint 3> mark @top;',
    'sub a by b; /* unsupported comment */',
    'sub a by b; } feature kern {',
    'pos A 32768;',
    'pos A -32769;',
    'pos A 1.5;',
    'pos A 1e2;',
    'pos A V -80;',
    'pos A 50;',
  ])('rejects unsupported source in full: %s', (source) => {
    expect(parseVisualLookup(source)).toBeNull()
  })

  it('bounds glyph lists and numeric fields', () => {
    const maxList = Array.from(
      { length: MAX_VISUAL_GLYPHS },
      (_, index) => `g${index}`,
    ).join(' ')
    const valid: VisualLookup = {
      kind: 'gpos-single',
      glyphs: maxList,
      value: { ...zero, xAdvance: -32768, yAdvance: 32767 },
    }
    expect(validateVisualLookup(valid)).toEqual([])
    expect(parseVisualLookup(serializeVisualLookup(valid))).toEqual(valid)
    const tooMany = { ...valid, glyphs: `${maxList} extra` }
    expect(validateVisualLookup(tooMany).length).toBeGreaterThan(0)
    expect(parseVisualLookup(serializeVisualLookup(tooMany))).toBeNull()
    for (const value of [NaN, Infinity, -32769, 32768, 1.5]) {
      const invalid = { ...valid, value: { ...zero, xAdvance: value } }
      expect(validateVisualLookup(invalid).length).toBeGreaterThan(0)
    }
  })

  it('validates class names, glyph identifiers, arity and anchors', () => {
    const invalid: VisualLookup[] = [
      { kind: 'gsub-single', from: 'a b c', to: 'd e' },
      { kind: 'gsub-single', from: '', to: 'a' },
      { kind: 'gsub-single', from: 'a; sub b by c', to: 'a' },
      { kind: 'gsub-single', from: '字', to: 'a' },
      { kind: 'gsub-single', from: 'a'.repeat(64), to: 'a' },
      { kind: 'gsub-multiple', from: 'a b', to: 'c d' },
      { kind: 'gsub-multiple', from: 'a', to: 'b' },
      { kind: 'gsub-ligature', from: 'a', to: 'b' },
      { kind: 'gsub-ligature', from: 'a b', to: 'c d' },
      {
        kind: 'gsub-contextual',
        before: '',
        input: 'a b',
        after: '',
        replacement: 'c',
      },
      {
        kind: 'gpos-mark-base',
        markGlyphs: 'acute',
        markClass: '@bad',
        markAnchor: anchor,
        targetGlyphs: 'A',
        targetAnchor: anchor,
      },
      {
        kind: 'gpos-mark-mark',
        markGlyphs: 'acute',
        markClass: 'top',
        markAnchor: { x: 0.5, y: 0 },
        targetGlyphs: 'grave',
        targetAnchor: anchor,
      },
    ]
    for (const rule of invalid)
      expect(validateVisualLookup(rule).length).toBeGreaterThan(0)
  })

  it('accepts exactly four safe ASCII feature tag bytes', () => {
    for (const tag of ['kern', 'ss01', 'T_ST'])
      expect(isValidFeatureTag(tag)).toBe(true)
    for (const tag of ['', 'abc', 'abcde', '汉字标签', 'a\ncd', 'a};{', 'a bc'])
      expect(isValidFeatureTag(tag)).toBe(false)
  })
})
