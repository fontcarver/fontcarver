import {
  validateVisualLookup,
  VISUAL_CLASS_NAME,
  VISUAL_GLYPH_NAME,
  type VisualAnchor,
  type VisualLookup,
  type VisualValueRecord,
} from './visualLookup'

// Unescaped reserved words, ranges, class references and CID glyphs need a
// broader model. Refusing them avoids silently changing source semantics.
const reserved = new Set(
  'anchor anchorDef anon anonymous base by contourpoint cursive device enum enumerate exclude_dflt feature from ignore include include_dflt language languagesystem ligature lookup lookupflag mark markClass nameid parameters pos position required reversesub rsub script sub substitute subtable table useExtension valueRecordDef RightToLeft IgnoreBaseGlyphs IgnoreLigatures IgnoreMarks MarkAttachmentType UseMarkFilteringSet NULL'.split(
    ' ',
  ),
)
type GlyphOperand = { names: string; isClass: boolean }

class SupportedParser {
  private index = 0
  constructor(private readonly tokens: string[]) {}

  peek(): string | undefined {
    return this.tokens[this.index]
  }
  take(): string {
    return this.tokens[this.index++] ?? ''
  }
  eat(token: string): boolean {
    if (this.peek() !== token) return false
    this.index += 1
    return true
  }
  expect(token: string): void {
    if (!this.eat(token)) throw new Error('Unsupported FEA')
  }
  glyph(): string {
    const token = this.take()
    const escaped = token.startsWith('\\')
    const name = escaped ? token.slice(1) : token
    if (
      !VISUAL_GLYPH_NAME.test(name) ||
      (!escaped && (reserved.has(name) || name.includes('-')))
    )
      throw new Error('Unsupported glyph')
    return name
  }
  operand(): GlyphOperand {
    if (!this.eat('[')) return { names: this.glyph(), isClass: false }
    const names: string[] = []
    while (this.peek() !== ']' && this.peek() !== undefined)
      names.push(this.glyph())
    this.expect(']')
    if (names.length === 0) throw new Error('Empty class')
    return { names: names.join(' '), isClass: true }
  }
  integer(): number {
    const value = this.take()
    if (!/^[+-]?\d+$/.test(value)) throw new Error('Unsupported number')
    return Number(value)
  }
  value(): VisualValueRecord {
    // Scalars depend on the enclosing feature (e.g. vkrn); do not guess axes.
    this.expect('<')
    const value = {
      xPlacement: this.integer(),
      yPlacement: this.integer(),
      xAdvance: this.integer(),
      yAdvance: this.integer(),
    }
    this.expect('>')
    return value
  }
  anchor(): VisualAnchor {
    this.expect('<')
    this.expect('anchor')
    const value = { x: this.integer(), y: this.integer() }
    this.expect('>')
    return value
  }
  className(): string {
    const token = this.take()
    if (!token.startsWith('@') || !VISUAL_CLASS_NAME.test(token.slice(1)))
      throw new Error('Unsupported class name')
    return token.slice(1)
  }
  substitution(): VisualLookup {
    const inputs: (GlyphOperand & { marked: boolean })[] = []
    while (this.peek() !== 'by' && this.peek() !== 'from') {
      inputs.push({ ...this.operand(), marked: this.eat("'") })
    }
    const operator = this.take()
    const output: GlyphOperand[] = []
    while (this.peek() !== ';') output.push(this.operand())
    this.expect(';')
    if (!inputs.length || !output.length) throw new Error('Empty substitution')
    if (operator === 'from') {
      if (
        inputs.length !== 1 ||
        inputs[0].isClass ||
        inputs[0].marked ||
        output.length !== 1 ||
        !output[0].isClass
      )
        throw new Error('Unsupported alternate')
      return {
        kind: 'gsub-alternate',
        from: inputs[0].names,
        to: output[0].names,
      }
    }
    const marked = inputs.findIndex((input) => input.marked)
    if (marked >= 0) {
      if (
        inputs.some(
          (input, index) => input.isClass || (input.marked && index !== marked),
        ) ||
        output.length !== 1 ||
        output[0].isClass
      )
        throw new Error('Unsupported contextual substitution')
      return {
        kind: 'gsub-contextual',
        before: inputs
          .slice(0, marked)
          .map((input) => input.names)
          .join(' '),
        input: inputs[marked].names,
        after: inputs
          .slice(marked + 1)
          .map((input) => input.names)
          .join(' '),
        replacement: output[0].names,
      }
    }
    if (inputs.length === 1 && output.length === 1) {
      // A singleton destination class cannot stand for a repeated glyph.
      if (
        output[0].isClass &&
        inputs[0].names.split(' ').length !== output[0].names.split(' ').length
      )
        throw new Error('Unmatched classes')
      return { kind: 'gsub-single', from: inputs[0].names, to: output[0].names }
    }
    if (
      inputs.some((input) => input.isClass) ||
      output.some((item) => item.isClass)
    )
      throw new Error('Unsupported classes')
    if (inputs.length === 1)
      return {
        kind: 'gsub-multiple',
        from: inputs[0].names,
        to: output.map((item) => item.names).join(' '),
      }
    if (output.length === 1)
      return {
        kind: 'gsub-ligature',
        from: inputs.map((item) => item.names).join(' '),
        to: output[0].names,
      }
    throw new Error('Unsupported substitution')
  }
  positioning(): VisualLookup {
    const first = this.operand().names
    const firstValue = this.value()
    if (this.eat(';'))
      return { kind: 'gpos-single', glyphs: first, value: firstValue }
    const second = this.operand().names
    const secondValue = this.value()
    this.expect(';')
    return { kind: 'gpos-pair', first, second, firstValue, secondValue }
  }
  attachment(): VisualLookup {
    const markGlyphs = this.operand().names
    const markAnchor = this.anchor()
    const markClass = this.className()
    this.expect(';')
    if (!this.eat('pos')) this.expect('position')
    const attachment = this.take()
    if (attachment !== 'base' && attachment !== 'mark')
      throw new Error('Unsupported attachment')
    const targetGlyphs = this.operand().names
    const targetAnchor = this.anchor()
    this.expect('mark')
    if (this.className() !== markClass) throw new Error('Unmatched mark class')
    this.expect(';')
    return {
      kind: attachment === 'base' ? 'gpos-mark-base' : 'gpos-mark-mark',
      markGlyphs,
      markAnchor,
      markClass,
      targetGlyphs,
      targetAnchor,
    }
  }
}

/** Recognize a complete bounded feature body; unsupported FEA remains in source mode. */
export function parseVisualLookup(code: string): VisualLookup | null {
  if (code.length > 65536) return null
  const tokens: string[] = []
  const pattern =
    /\s+|#[^\r\n]*|\\[A-Za-z_.][A-Za-z0-9_.-]*|@[A-Za-z_][A-Za-z0-9_.-]*|[A-Za-z_.][A-Za-z0-9_.-]*|[+-]?\d+|[[\]<>;']/gy
  let index = 0
  while (index < code.length) {
    const match = pattern.exec(code)
    if (!match) return null
    index = pattern.lastIndex
    const token = match[0]
    if (/^\s/.test(token) || token.startsWith('#')) continue
    tokens.push(token)
    if (tokens.length > 4096) return null
  }
  try {
    const parser = new SupportedParser(tokens)
    const keyword = parser.take()
    let rule: VisualLookup
    if (keyword === 'sub' || keyword === 'substitute')
      rule = parser.substitution()
    else if (keyword === 'pos' || keyword === 'position')
      rule = parser.positioning()
    else if (keyword === 'markClass') rule = parser.attachment()
    else return null
    return parser.peek() === undefined &&
      validateVisualLookup(rule).length === 0
      ? rule
      : null
  } catch {
    return null
  }
}
