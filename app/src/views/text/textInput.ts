/** A piece of text that can be sent to the shaping kernel unchanged. */
export interface LiteralTextToken {
  kind: 'literal'
  text: string
  /** UTF-16 indexes in the original input, suitable for DOM selection. */
  sourceStart: number
  sourceEnd: number
  /** UTF-8 byte offset in the original input, matching shaping clusters. */
  sourceByteStart: number
  /** Maps each shaped UTF-8 byte boundary back to the original input. */
  sourceByteMap: number[]
}

/** An exact document glyph name introduced by slash syntax. */
export interface NamedGlyphToken {
  kind: 'named'
  name: string
  sourceStart: number
  sourceEnd: number
  sourceByteStart: number
}

export type TextInputToken = LiteralTextToken | NamedGlyphToken

const utf8Length = (value: string): number =>
  new TextEncoder().encode(value).length

/**
 * Splits editor text into shapeable Unicode spans and exact `/name` tokens.
 *
 * A name consumes one trailing whitespace character as its delimiter, matching
 * the common `/A /B` spelling without inserting spaces in the run. A second
 * whitespace remains literal. `//` spells one literal slash, and a bare or
 * trailing slash is literal too. A byte-boundary map accounts for the removed
 * escape slash while keeping the whole literal span available to the shaper.
 */
export function parseTextInput(input: string): TextInputToken[] {
  const tokens: TextInputToken[] = []
  let literal: LiteralTextToken | null = null

  const appendDirect = (start: number, end: number): void => {
    if (start === end) return
    const sourceByteStart = utf8Length(input.slice(0, start))
    const text = input.slice(start, end)
    if (!literal) {
      literal = {
        kind: 'literal',
        text: '',
        sourceStart: start,
        sourceEnd: start,
        sourceByteStart,
        sourceByteMap: [sourceByteStart],
      }
    }
    literal.text += text
    literal.sourceEnd = end
    const byteLength = utf8Length(text)
    for (let byte = 1; byte <= byteLength; byte += 1)
      literal.sourceByteMap.push(sourceByteStart + byte)
  }

  const appendEscapedSlash = (start: number): void => {
    const sourceByteStart = utf8Length(input.slice(0, start))
    if (!literal) {
      literal = {
        kind: 'literal',
        text: '',
        sourceStart: start,
        sourceEnd: start,
        sourceByteStart,
        sourceByteMap: [sourceByteStart],
      }
    }
    literal.text += '/'
    literal.sourceEnd = start + 2
    literal.sourceByteMap.push(sourceByteStart + 2)
  }

  const flushLiteral = (): void => {
    if (literal) tokens.push(literal)
    literal = null
  }

  let literalStart = 0
  let index = 0
  while (index < input.length) {
    if (input[index] !== '/') {
      index += 1
      continue
    }

    appendDirect(literalStart, index)
    if (input[index + 1] === '/') {
      appendEscapedSlash(index)
      index += 2
      literalStart = index
      continue
    }

    let end = index + 1
    while (end < input.length && !/\s/u.test(input[end] ?? '')) end += 1
    if (end === index + 1) {
      appendDirect(index, index + 1)
      index += 1
      literalStart = index
      continue
    }

    const sourceEnd = end < input.length ? end + 1 : end
    flushLiteral()
    tokens.push({
      kind: 'named',
      name: input.slice(index + 1, end),
      sourceStart: index,
      sourceEnd,
      sourceByteStart: utf8Length(input.slice(0, index)),
    })
    index = sourceEnd
    literalStart = index
  }

  appendDirect(literalStart, input.length)
  flushLiteral()
  return tokens
}
