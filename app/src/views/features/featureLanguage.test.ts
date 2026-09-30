import { CompletionContext } from '@codemirror/autocomplete'
import { syntaxTree } from '@codemirror/language'
import { EditorState } from '@codemirror/state'
import { describe, expect, it, vi } from 'vitest'

import {
  createFeatureCompletion,
  featureClassAt,
  featureLanguage,
  sourceOffsetToEditorOffset,
  sourceOffsetToLocation,
} from './featureLanguage'

describe('feature language', () => {
  it('identifies class references at the cursor while excluding escaped names and prose', () => {
    const doc =
      'sub @Upper-case.alt by A;\n# @Comment\nnameid 1 "@String";\n\\@Escaped'
    const state = EditorState.create({ doc, extensions: [featureLanguage] })
    for (const position of [doc.indexOf('@Upper'), doc.indexOf(' by')]) {
      expect(featureClassAt(state, position)).toBe('Upper-case.alt')
    }
    for (const text of ['Comment', 'String', 'Escaped', 'by']) {
      expect(featureClassAt(state, doc.indexOf(text) + 1)).toBeNull()
    }
  })

  it('recognizes FEA comments, classes, escaped glyphs and keywords', () => {
    const state = EditorState.create({
      doc: '# 中文\n@Upper = [A \\feature];\nfeature kern { pos A V -80; } kern;',
      extensions: [featureLanguage],
    })
    const names: string[] = []
    syntaxTree(state).iterate({
      enter: (node) => {
        names.push(node.name)
      },
    })
    expect(names).toContain('comment')
    expect(names).toContain('variableName')
    expect(names).toContain('keyword')
    expect(names).toContain('number')
  })

  it('offers glyphs, classes and feature tags without duplicate entries', () => {
    const complete = createFeatureCompletion(
      ['A', 'A.alt', 'A'],
      ['Upper', '@Upper'],
    )
    const state = EditorState.create({
      doc: '@Up',
      extensions: [featureLanguage],
    })
    const result = complete(new CompletionContext(state, 3, true))
    expect(result?.from).toBe(0)
    expect(result?.options.map((option) => option.label)).toEqual(['@Upper'])

    const glyphState = EditorState.create({
      doc: 'sub A',
      extensions: [featureLanguage],
    })
    const glyphs = complete(new CompletionContext(glyphState, 5, true))
    expect(
      glyphs?.options.filter((option) => option.label === 'A'),
    ).toHaveLength(1)
    expect(glyphs?.options.some((option) => option.label === 'A.alt')).toBe(
      true,
    )
    expect(glyphs?.options.some((option) => option.label === 'liga')).toBe(true)
  })

  it('does not offer names inside comments or quoted strings', () => {
    const complete = createFeatureCompletion(['A'], ['Upper'])
    for (const doc of ['# comment A', 'nameid 1 "A']) {
      const state = EditorState.create({ doc, extensions: [featureLanguage] })
      expect(
        complete(new CompletionContext(state, doc.length, true)),
      ).toBeNull()
    }
  })

  it('maps UTF-16 diagnostic offsets across CRLF without corrupting emoji', () => {
    const source = '# 字😀\r\npos A V -80;\r\n'
    expect(sourceOffsetToEditorOffset(source, source.indexOf('A'))).toBe(10)
    expect(sourceOffsetToEditorOffset(source, -10)).toBe(0)
    expect(sourceOffsetToEditorOffset(source, 100)).toBe(source.length - 2)
  })

  it('reports one-based source locations across newline forms and Unicode characters', () => {
    const source = '# setup\r\n# 字😀 invalid\rsub A by B;\n'
    expect(sourceOffsetToLocation(source, source.indexOf('invalid'))).toEqual({
      line: 2,
      column: 6,
    })
    expect(sourceOffsetToLocation(source, source.indexOf('sub'))).toEqual({
      line: 3,
      column: 1,
    })
    expect(sourceOffsetToLocation(source, -1)).toEqual({ line: 1, column: 1 })
    expect(sourceOffsetToLocation(source, source.length + 100)).toEqual({
      line: 4,
      column: 1,
    })
  })

  it('does not treat contextual glyph marks as paired apostrophes', () => {
    const state = EditorState.create({
      doc: "sub A' B by C;",
      extensions: [featureLanguage],
    })
    const config = state.languageDataAt<{ brackets: string[] }>(
      'closeBrackets',
      5,
    )
    expect(config[0].brackets).not.toContain("'")
    expect(config[0].brackets).toContain('[')
  })

  it('queries glyphs beyond the initial list and refreshes after each token change', async () => {
    const loadGlyphNames = vi.fn().mockResolvedValue(['uni9FA5', 'uni9FA5.alt'])
    const complete = createFeatureCompletion(['A'], ['Upper'], loadGlyphNames)
    const state = EditorState.create({
      doc: 'sub uni9',
      extensions: [featureLanguage],
    })
    const result = await complete(
      new CompletionContext(state, state.doc.length, true),
    )
    expect(loadGlyphNames).toHaveBeenCalledExactlyOnceWith('uni9')
    expect(result?.options.some((option) => option.label === 'uni9FA5')).toBe(
      true,
    )
    expect(result?.options.some((option) => option.label === 'A')).toBe(true)
    expect(typeof result?.validFor).toBe('function')
    if (typeof result?.validFor === 'function') {
      expect(result.validFor('uni9', 4, 8, state)).toBe(true)
      expect(result.validFor('uni9F', 4, 9, state)).toBe(false)
    }
  })

  it('ignores a delayed result after a newer query starts', async () => {
    let resolveOld: (names: readonly string[]) => void = () => undefined
    const oldQuery = new Promise<readonly string[]>((resolve) => {
      resolveOld = resolve
    })
    const loadGlyphNames = vi
      .fn()
      .mockReturnValueOnce(oldQuery)
      .mockResolvedValueOnce(['uni9FA5'])
    const complete = createFeatureCompletion([], [], loadGlyphNames)
    const oldState = EditorState.create({
      doc: 'uni8',
      extensions: [featureLanguage],
    })
    const nextState = EditorState.create({
      doc: 'uni9',
      extensions: [featureLanguage],
    })
    const stale = complete(new CompletionContext(oldState, 4, true))
    const current = complete(new CompletionContext(nextState, 4, true))
    expect(
      (await current)?.options.some((option) => option.label === 'uni9FA5'),
    ).toBe(true)
    resolveOld(['uni8ABC'])
    expect(await stale).toBeNull()
  })

  it('uses local class names and survives a glyph query failure', async () => {
    const loadGlyphNames = vi
      .fn()
      .mockRejectedValue(new Error('document closed'))
    const complete = createFeatureCompletion(
      ['A.alt'],
      ['Upper'],
      loadGlyphNames,
    )
    const classState = EditorState.create({
      doc: '@Up',
      extensions: [featureLanguage],
    })
    expect(
      (await complete(new CompletionContext(classState, 3, true)))?.options.map(
        (option) => option.label,
      ),
    ).toEqual(['@Upper'])
    expect(loadGlyphNames).not.toHaveBeenCalled()
    const glyphState = EditorState.create({
      doc: '\\A',
      extensions: [featureLanguage],
    })
    const result = await complete(new CompletionContext(glyphState, 2, true))
    expect(loadGlyphNames).toHaveBeenCalledExactlyOnceWith('A')
    expect(result?.options.map((option) => option.label)).toEqual(['\\A.alt'])
  })
})

describe('feature dialect tokens and conditionals', () => {
  const tokenNames = (doc: string) => {
    const state = EditorState.create({ doc, extensions: [featureLanguage] })
    const names: string[] = []
    syntaxTree(state).iterate({
      enter: (node) => {
        names.push(node.name)
      },
    })
    return names
  }

  it('marks whole-line target conditionals apart from comments', () => {
    const names = tokenNames(
      '#ifdef VARIABLE\npos A -10;\n#else\npos A -20;\n#endif\n# an ordinary comment\n',
    )
    expect(names.filter((name) => name === 'meta')).toHaveLength(3)
    expect(names).toContain('comment')
  })

  it('does not treat a trailing hash as a directive', () => {
    expect(tokenNames('pos A -10; #ifdef VARIABLE')).not.toContain('meta')
  })

  it('highlights number, expression and predicate tokens', () => {
    const names = tokenNames(
      "pos A $unitsPerEm;\npos A ${-A:width / 10};\nsub [$[name ENDSWITH '.sc']] by A;\n",
    )
    expect(
      names.filter((name) => name === 'atom').length,
    ).toBeGreaterThanOrEqual(3)
    expect(names).not.toContain('invalid')
  })

  it('marks an unterminated token as invalid instead of plain source', () => {
    expect(tokenNames('pos A ${A:width;')).toContain('invalid')
  })

  it('completes directives and built-in metric tokens', () => {
    const complete = createFeatureCompletion(['A'], [])
    const directive = EditorState.create({
      doc: '#if',
      extensions: [featureLanguage],
    })
    const directives = complete(new CompletionContext(directive, 3, true))
    expect(directives?.from).toBe(0)
    expect(directives?.options.map((option) => option.label)).toContain(
      '#ifdef VARIABLE',
    )
    // A complete directive closes the popup so Enter still inserts a newline.
    const finished = EditorState.create({
      doc: '#endif',
      extensions: [featureLanguage],
    })
    expect(complete(new CompletionContext(finished, 6, true))).toBeNull()

    const token = EditorState.create({
      doc: 'pos A $un',
      extensions: [featureLanguage],
    })
    const tokens = complete(new CompletionContext(token, 9, true))
    expect(tokens?.from).toBe(6)
    expect(tokens?.options.map((option) => option.label)).toContain(
      '$unitsPerEm',
    )
    expect(tokens?.options.map((option) => option.label)).not.toContain('liga')
  })
})
