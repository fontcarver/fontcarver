import { describe, expect, it } from 'vitest'

import { parseTextInput } from './textInput'

describe('parseTextInput', () => {
  it('keeps Unicode text in contiguous spans around named glyph escapes', () => {
    const parsed = parseTextInput('office عربي/A.alt /acutecomb 世界')
    expect(
      parsed.map((token) => {
        if (token.kind === 'named') return token
        return {
          kind: token.kind,
          text: token.text,
          sourceStart: token.sourceStart,
          sourceEnd: token.sourceEnd,
          sourceByteStart: token.sourceByteStart,
        }
      }),
    ).toEqual([
      {
        kind: 'literal',
        text: 'office عربي',
        sourceStart: 0,
        sourceEnd: 11,
        sourceByteStart: 0,
      },
      {
        kind: 'named',
        name: 'A.alt',
        sourceStart: 11,
        sourceEnd: 18,
        sourceByteStart: 15,
      },
      {
        kind: 'named',
        name: 'acutecomb',
        sourceStart: 18,
        sourceEnd: 29,
        sourceByteStart: 22,
      },
      {
        kind: 'literal',
        text: '世界',
        sourceStart: 29,
        sourceEnd: 31,
        sourceByteStart: 33,
      },
    ])
  })

  it('uses one whitespace delimiter and documents literal slash spelling', () => {
    const parsed = parseTextInput('/A  B//C/')
    expect(
      parsed.map((token) => {
        if (token.kind === 'named') return token
        return {
          kind: token.kind,
          text: token.text,
          sourceStart: token.sourceStart,
          sourceEnd: token.sourceEnd,
          sourceByteStart: token.sourceByteStart,
        }
      }),
    ).toEqual([
      {
        kind: 'named',
        name: 'A',
        sourceStart: 0,
        sourceEnd: 3,
        sourceByteStart: 0,
      },
      {
        kind: 'literal',
        text: ' B/C/',
        sourceStart: 3,
        sourceEnd: 9,
        sourceByteStart: 3,
      },
    ])
    expect(parsed[1]?.kind === 'literal' && parsed[1].sourceByteMap).toEqual([
      3, 4, 5, 7, 8, 9,
    ])
  })

  it('maps clusters after Unicode and an escaped slash to source bytes', () => {
    const [token] = parseTextInput('é//A')
    expect(token?.kind).toBe('literal')
    expect(token?.kind === 'literal' && token.text).toBe('é/A')
    expect(token?.kind === 'literal' && token.sourceByteMap).toEqual([
      0, 1, 2, 4, 5,
    ])
  })
})
