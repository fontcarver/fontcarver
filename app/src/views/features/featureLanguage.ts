import type {
  Completion,
  CompletionContext,
  CompletionResult,
  CompletionSource,
} from '@codemirror/autocomplete'
import { StreamLanguage, syntaxTree } from '@codemirror/language'
import type { EditorState } from '@codemirror/state'

const KEYWORDS = new Set([
  'anchor',
  'anchorDef',
  'anon',
  'anonymous',
  'by',
  'cursive',
  'enum',
  'enumerate',
  'exclude_dflt',
  'feature',
  'from',
  'ignore',
  'include',
  'include_dflt',
  'language',
  'languagesystem',
  'lookup',
  'lookupflag',
  'mark',
  'markClass',
  'nameid',
  'parameters',
  'pos',
  'position',
  'required',
  'reversesub',
  'rsub',
  'script',
  'sub',
  'substitute',
  'subtable',
  'table',
  'useExtension',
  'valueRecordDef',
  'RightToLeft',
  'IgnoreBaseGlyphs',
  'IgnoreLigatures',
  'IgnoreMarks',
  'MarkAttachmentType',
  'UseMarkFilteringSet',
  'NULL',
])

/** Whole-line target conditionals of the FontCarver feature dialect. */
const DIRECTIVES = ['#ifdef VARIABLE', '#ifndef VARIABLE', '#else', '#endif']
const DIRECTIVE_START = /^#(?:ifdef|ifndef|else|endif)\b/
/** Built-in metric tokens the kernel resolves at compile time. */
const METRIC_TOKENS = [
  '$unitsPerEm',
  '$ascender',
  '$descender',
  '$xHeight',
  '$capHeight',
  '$italicAngle',
]
/** Token shapes: a named number, a numeric expression, a glyph predicate. */
const TOKEN_NAME = /\$[A-Za-z_][A-Za-z0-9_]*/
const TOKEN_EXPRESSION = /\$\{[^}\r\n]*\}/
const TOKEN_PREDICATE = /\$\[[^\]\r\n]*\]/

const FEATURE_TAGS = [
  'aalt',
  'calt',
  'ccmp',
  'clig',
  'cpsp',
  'curs',
  'dlig',
  'frac',
  'kern',
  'liga',
  'locl',
  'mark',
  'mkmk',
  'numr',
  'onum',
  'pnum',
  'rlig',
  'salt',
  'smcp',
  'tnum',
  'vert',
  'vkrn',
  'zero',
  ...Array.from(
    { length: 20 },
    (_, index) => `ss${String(index + 1).padStart(2, '0')}`,
  ),
]

/** Lightweight FEA highlighting; authoritative parsing stays in the kernel. */
export const featureLanguage = StreamLanguage.define({
  startState: () => ({ quoted: false }),
  token(stream, state) {
    if (state.quoted) {
      let escaped = false
      while (!stream.eol()) {
        const character = stream.next()
        if (character === '"' && !escaped) {
          state.quoted = false
          break
        }
        escaped = !escaped && character === '\\'
      }
      return 'string'
    }
    if (stream.eatSpace()) return null
    if (stream.peek() === '#') {
      // A conditional directive occupies its whole line; everything else
      // beginning with a hash is an ordinary comment.
      const lineStart = /^\s*$/.test(stream.string.slice(0, stream.start))
      const rest = stream.string.slice(stream.start)
      stream.skipToEnd()
      return lineStart && DIRECTIVE_START.test(rest) ? 'meta' : 'comment'
    }
    if (stream.eat('"')) {
      state.quoted = true
      return 'string'
    }
    if (stream.peek() === '$') {
      if (
        stream.match(TOKEN_EXPRESSION) ||
        stream.match(TOKEN_PREDICATE) ||
        stream.match(TOKEN_NAME)
      )
        return 'atom'
      // An unterminated token must not be mistaken for ordinary source.
      stream.next()
      return 'invalid'
    }
    if (stream.match(/@[\w.-]+/)) return 'variableName'
    if (stream.match(/\\[\w.-]+/)) return 'variableName'
    if (stream.match(/[-+]?\d+(?:\.\d+)?/)) return 'number'
    if (stream.match(/[\w.-]+/)) {
      const word = stream.current()
      return KEYWORDS.has(word) ? 'keyword' : 'variableName'
    }
    stream.next()
    return 'punctuation'
  },
  languageData: {
    commentTokens: { line: '#' },
    // Apostrophes mark contextual glyphs in FEA; they are not string delimiters.
    closeBrackets: { brackets: ['(', '[', '{', '"'] },
  },
})

/** Resolve a source class reference while excluding comments and strings. */
export function featureClassAt(
  state: EditorState,
  position: number,
): string | null {
  const token = syntaxTree(state).resolveInner(position, -1)
  if (token.name === 'comment' || token.name === 'string') return null
  const line = state.doc.lineAt(position)
  for (const match of line.text.matchAll(/@[\w.-]+/g)) {
    const from = line.from + match.index
    if (from <= position && position <= from + match[0].length) {
      if (match.index > 0 && /[\\\w.-]/.test(line.text[match.index - 1]!))
        return null
      return match[0].slice(1)
    }
  }
  return null
}

/** Build name completions without inserting translated prose into source code. */
export function createFeatureCompletion(
  glyphNames: readonly string[],
  classNames: readonly string[],
): (context: CompletionContext) => CompletionResult | null
/** Query the font for glyph names beyond the initial bounded name list. */
export function createFeatureCompletion(
  glyphNames: readonly string[],
  classNames: readonly string[],
  loadGlyphNames: ((query: string) => Promise<readonly string[]>) | undefined,
): CompletionSource
export function createFeatureCompletion(
  glyphNames: readonly string[],
  classNames: readonly string[],
  loadGlyphNames?: (query: string) => Promise<readonly string[]>,
): CompletionSource {
  const classes: Completion[] = [
    ...new Set(classNames.map((name) => `@${name.replace(/^@/, '')}`)),
  ].map((label) => ({ label, type: 'variable' }))
  const tokens: Completion[] = METRIC_TOKENS.map((label) => ({
    label,
    type: 'variable',
  }))
  const directives: Completion[] = DIRECTIVES.map((label) => ({
    label,
    type: 'keyword',
  }))
  const names = new Map<string, Completion>()
  for (const label of KEYWORDS) names.set(label, { label, type: 'keyword' })
  for (const label of FEATURE_TAGS)
    names.set(label, { label, type: 'constant' })
  for (const label of glyphNames) names.set(label, { label, type: 'text' })
  for (const entry of classes) names.set(entry.label, entry)
  const options = [...names.values()]
  let queryRevision = 0
  return (context) => {
    const revision = ++queryRevision
    // A directive is being typed when the line so far is only a hash and
    // letters; the stream tokenizer still calls that a comment.
    const line = context.state.doc.lineAt(context.pos)
    const typed = line.text.slice(0, context.pos - line.from)
    if (/^\s*#[A-Za-z]*$/.test(typed)) {
      const word = typed.trimStart()
      // Once the directive is complete the popup closes, so Enter inserts a
      // newline instead of accepting the identical completion.
      const options = directives.filter(
        (option) => option.label !== word && option.label.startsWith(word),
      )
      if (options.length === 0) return null
      return {
        from: context.pos - word.length,
        options,
        validFor: /#[A-Za-z]*/,
      }
    }
    const token = syntaxTree(context.state).resolveInner(context.pos, -1)
    if (token.name === 'comment' || token.name === 'string') return null
    const word = context.matchBefore(/[@\\$]?[\w.-]*/)
    if (!word || (!context.explicit && word.from === word.to)) return null
    const escaped = word.text.startsWith('\\')
    if (word.text.startsWith('$'))
      return { from: word.from, options: tokens, validFor: /\$[\w]*/ }
    const result: CompletionResult = {
      from: word.from,
      options: word.text.startsWith('@')
        ? classes
        : escaped
          ? [...new Set(glyphNames)].map((name) => ({
              label: `\\${name}`,
              type: 'text',
            }))
          : options,
      validFor: /[@\\]?[\w.-]*/,
    }
    if (!loadGlyphNames || word.text.startsWith('@')) return result

    // A bounded response belongs to this exact query. Typing another character
    // must query again rather than filter an incomplete earlier result page.
    result.validFor = (text) => text === word.text
    let aborted = false
    context.addEventListener(
      'abort',
      () => {
        aborted = true
      },
      { onDocChange: true },
    )
    return Promise.resolve()
      .then(() => loadGlyphNames(escaped ? word.text.slice(1) : word.text))
      .then((loaded) => {
        const merged = new Map(
          result.options.map((option) => [option.label, option]),
        )
        for (const name of loaded) {
          const label = escaped ? `\\${name}` : name
          merged.set(label, { label, type: 'text' })
        }
        return { ...result, options: [...merged.values()] }
      })
      .catch(() => result)
      .then((response) =>
        aborted || context.aborted || revision !== queryRevision
          ? null
          : response,
      )
  }
}

/** Map a source UTF-16 offset to CodeMirror's normalized newline positions. */
export function sourceOffsetToEditorOffset(
  source: string,
  offset: number,
): number {
  const limit = Math.max(
    0,
    Math.min(source.length, Number.isFinite(offset) ? offset : 0),
  )
  let removed = 0
  for (let index = 0; index < limit; index += 1) {
    if (source[index] === '\r' && source[index + 1] === '\n') {
      removed += 1
      index += 1
    }
  }
  return limit - removed
}

/** Display one-based lines and Unicode character columns for a UTF-16 span. */
export function sourceOffsetToLocation(
  source: string,
  offset: number,
): { line: number; column: number } {
  const normalized = source.replace(/\r\n?/g, '\n')
  const position = sourceOffsetToEditorOffset(source, offset)
  const lines = normalized.slice(0, position).split('\n')
  return {
    line: lines.length,
    column: Array.from(lines.at(-1) ?? '').length + 1,
  }
}
