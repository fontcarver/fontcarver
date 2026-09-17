import { CompletionContext } from '@codemirror/autocomplete'
import { syntaxTree } from '@codemirror/language'
import { EditorState } from '@codemirror/state'
import { describe, expect, it, vi } from 'vitest'

import {
  createFeatureCompletion,
  featureLanguage,
  sourceOffsetToEditorOffset,
} from './featureLanguage'

describe('feature language', () => {
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
