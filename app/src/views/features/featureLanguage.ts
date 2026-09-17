import type {
  Completion,
  CompletionContext,
  CompletionResult,
  CompletionSource,
} from '@codemirror/autocomplete'
import { StreamLanguage, syntaxTree } from '@codemirror/language'

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
    if (stream.match(/#.*/)) return 'comment'
    if (stream.eat('"')) {
      state.quoted = true
      return 'string'
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
    const token = syntaxTree(context.state).resolveInner(context.pos, -1)
    if (token.name === 'comment' || token.name === 'string') return null
    const word = context.matchBefore(/[@\\]?[\w.-]*/)
    if (!word || (!context.explicit && word.from === word.to)) return null
    const escaped = word.text.startsWith('\\')
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
