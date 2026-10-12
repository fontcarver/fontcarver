import { describe, expect, it } from 'vitest'

import type { Features } from '../../api'

import { resolveFeatureClassMembers } from './featureClassMembers'

function source(classes: Record<string, string>): Features {
  return {
    prefixes: [],
    features: [],
    source_of_truth: 'Fea',
    classes: Object.entries(classes).map(([name, code]) => ({
      id: name,
      name,
      code,
      automatic: false,
    })),
  }
}

describe('resolveFeatureClassMembers', () => {
  it('resolves bare and bracketed classes, comments, escaped names and duplicate members', () => {
    expect(
      resolveFeatureClassMembers(
        source({
          Latin: 'A @Lower \\mark # ignored @Missing\r\n A',
          Lower: '[a b .notdef]',
        }),
        '@Latin',
      ),
    ).toEqual({
      members: ['A', 'a', 'b', '.notdef', 'mark'],
      issues: [],
      truncated: false,
    })
  })

  it('expands unambiguous letter and padded decimal ranges in source order', () => {
    expect(
      resolveFeatureClassMembers(
        source({ Letters: '[A - C a.sc - c.sc g008 - g011]' }),
        'Letters',
      ).members,
    ).toEqual([
      'A',
      'B',
      'C',
      'a.sc',
      'b.sc',
      'c.sc',
      'g008',
      'g009',
      'g010',
      'g011',
    ])
  })

  it('reports unresolved, cyclic and duplicate classes without selecting arbitrary definitions', () => {
    const input = source({
      A: '@B @Missing @Duplicate A',
      B: 'B @A',
      Duplicate: 'X',
    })
    input.classes.push({ ...input.classes[2], id: 'other', code: 'Y' })
    expect(resolveFeatureClassMembers(input, 'A')).toEqual({
      members: ['B', 'A'],
      issues: [
        { kind: 'cycle', name: 'A' },
        { kind: 'missing', name: 'Missing' },
        { kind: 'duplicate', name: 'Duplicate' },
      ],
      truncated: false,
    })
  })

  it.each([
    'A-Z',
    '[A $[category == "Letter"]]',
    '[A "fake"]',
    '[A',
    '[A] B',
    '\\123',
    '[Z - A]',
    '[a - zz]',
  ])('reports unsupported syntax instead of inventing members: %s', (code) => {
    expect(resolveFeatureClassMembers(source({ Bad: code }), 'Bad')).toEqual({
      members: [],
      issues: [{ kind: 'unsupported', name: 'Bad' }],
      truncated: false,
    })
  })

  it('bounds member expansion and explicitly reports an incomplete preview', () => {
    const answer = resolveFeatureClassMembers(
      source({ Many: 'g000 - g999' }),
      'Many',
    )
    expect(answer.members).toHaveLength(512)
    expect(answer.members[511]).toBe('g511')
    expect(answer.truncated).toBe(true)
    expect(answer.issues).toContainEqual({ kind: 'limit', name: 'Many' })
  })

  it('bounds recursion and avoids repeated expansion of a shared dependency', () => {
    const classes: Record<string, string> = { End: 'A' }
    for (let index = 40; index >= 0; index -= 1) {
      const next = index === 40 ? 'End' : `Class${index + 1}`
      classes[`Class${index}`] = `@${next} @${next}`
    }
    const answer = resolveFeatureClassMembers(source(classes), 'Class0')
    expect(answer.truncated).toBe(true)
    expect(answer.issues.some((issue) => issue.kind === 'limit')).toBe(true)
  })

  it('does not confuse object property names with class definitions', () => {
    expect(
      resolveFeatureClassMembers(source({ A: '@constructor' }), 'A').issues,
    ).toEqual([{ kind: 'missing', name: 'constructor' }])
  })
})
