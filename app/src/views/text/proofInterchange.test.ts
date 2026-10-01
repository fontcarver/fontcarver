import { describe, expect, it } from 'vitest'

import {
  exportProofText,
  importProofText,
  PROOF_INTERCHANGE_HEADER,
} from './proofInterchange'
import { ProofLibraryError, type ProofEntry } from './proofLibrary'
import {
  BUILTIN_PROOF_SAMPLES,
  builtinProofEntries,
  withBuiltinSamples,
} from './proofSamples'

function entry(overrides: Partial<ProofEntry> = {}): ProofEntry {
  return {
    id: 'one',
    name: 'Pangram',
    group: 'Latin',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'The quick brown fox.',
    ...overrides,
  }
}

let counter = 0
const makeId = () => `new-${++counter}`

describe('proof library text interchange', () => {
  it('round-trips every field, including tabs and newlines in the text', () => {
    const entries = [
      entry(),
      entry({
        id: 'two',
        name: 'Vertical',
        group: 'CJK',
        script: 'Hani',
        language: 'zh-hans',
        direction: 'ttb',
        text: '永\t東\n國\\道',
      }),
    ]
    const text = exportProofText(entries)
    expect(text.startsWith(`${PROOF_INTERCHANGE_HEADER}\n`)).toBe(true)
    expect(text).toContain('永\\t東\\n國\\\\道')
    const imported = importProofText(text, entries, makeId)
    expect(imported).toEqual(entries)
  })

  it('updates an entry whose group and name match and adds the rest', () => {
    const existing = [entry()]
    const text = exportProofText([
      entry({ text: 'Corrected text.' }),
      entry({ id: 'ignored', name: 'Pairs', text: 'AV AW AY' }),
    ])
    const imported = importProofText(text, existing, makeId)
    expect(imported).toHaveLength(2)
    expect(imported[0]).toMatchObject({ id: 'one', text: 'Corrected text.' })
    expect(imported[1]).toMatchObject({ name: 'Pairs', id: 'new-1' })
  })

  it('keeps unrelated stored entries and never duplicates an identifier', () => {
    const existing = [entry(), entry({ id: 'other', name: 'Other' })]
    const imported = importProofText(
      exportProofText([entry({ text: 'Changed.' })]),
      existing,
      makeId,
    )
    expect(imported.map((item) => item.id)).toEqual(['other', 'one'])
    expect(new Set(imported.map((item) => item.id)).size).toBe(2)
  })

  it("ignores comments and blank lines but keeps a text's trailing spaces", () => {
    const imported = importProofText(
      `# a comment\n\n   \nLatin\tPangram\tLatn\ten\tltr\tHH HOH  \n`,
      [],
      makeId,
    )
    // A spacing string's trailing spaces are part of the proof.
    expect(imported).toEqual([entry({ id: 'new-2', text: 'HH HOH  ' })])
  })

  it('round-trips a text made only of spaces', () => {
    const spaces = entry({ text: '   ' })
    expect(
      importProofText(exportProofText([spaces]), [spaces], makeId),
    ).toEqual([spaces])
  })

  it('round-trips an entry whose group or name starts with a hash', () => {
    const hashed = [
      entry({ id: 'one', group: '#Draft', name: 'Pangram' }),
      entry({ id: 'two', group: 'Latin', name: '#Working', text: 'AV' }),
      entry({ id: 'three', group: 'Latin', name: 'Hash', text: 'C# F#' }),
    ]
    const text = exportProofText(hashed)
    // The comment marker is escaped only where it would start a line; a hash
    // inside a field stays readable.
    expect(text).toContain('\\#Draft')
    expect(text).toContain('C# F#')
    expect(importProofText(text, hashed, makeId)).toEqual(hashed)
    // Nothing is silently dropped: all three come back.
    expect(importProofText(text, [], makeId)).toHaveLength(3)
  })

  it('rejects a whole document with a malformed line, leaving nothing imported', () => {
    for (const bad of [
      'Latin\tPangram\tLatn\ten\tltr',
      'Latin\tPangram\tLatn\ten\tltr\ttext\textra',
      'Latin\tPangram\tLatn\ten\tsideways\ttext',
      'Latin\t\tLatn\ten\tltr\ttext',
      'Latin\tPangram\tLatn\ten\tltr\t',
      'Latin\tPangram\tLatn\ten\tltr\tbad \\x escape',
      '',
      '# only a comment\n',
    ])
      expect(() => importProofText(bad, [], makeId)).toThrow(ProofLibraryError)
  })

  it('rejects two lines that would collide on group and name', () => {
    expect(() =>
      importProofText(
        'Latin\tPangram\tLatn\ten\tltr\tone\nLatin\tPangram\tLatn\ten\tltr\ttwo',
        [],
        makeId,
      ),
    ).toThrow(ProofLibraryError)
  })

  it('rejects an import that would exceed the library bound', () => {
    const many = Array.from({ length: 257 }, (_, index) =>
      entry({ id: `id-${index}`, name: `Name ${index}` }),
    )
    expect(() => importProofText(exportProofText(many), [], makeId)).toThrow(
      ProofLibraryError,
    )
  })
})

describe('built-in proof samples', () => {
  const samples = builtinProofEntries((key) => key)

  it('passes the library validator and has unique stable identifiers', () => {
    expect(() =>
      importProofText(exportProofText(samples), [], makeId),
    ).not.toThrow()
    expect(new Set(samples.map((sample) => sample.id)).size).toBe(
      samples.length,
    )
    expect(samples.every((sample) => sample.id.startsWith('fc.sample.'))).toBe(
      true,
    )
  })

  it('covers Latin, Cyrillic, Greek, CJK and Arabic with pairs and phrases', () => {
    const scripts = new Set(BUILTIN_PROOF_SAMPLES.map((item) => item.script))
    expect([...scripts].sort()).toEqual([
      'Arab',
      'Cyrl',
      'Grek',
      'Hani',
      'Latn',
    ])
    const groups = new Set(BUILTIN_PROOF_SAMPLES.map((item) => item.groupKey))
    expect(groups.has('latinPairs')).toBe(true)
    expect(BUILTIN_PROOF_SAMPLES.some((item) => item.direction === 'ttb')).toBe(
      true,
    )
    expect(BUILTIN_PROOF_SAMPLES.some((item) => item.direction === 'rtl')).toBe(
      true,
    )
  })

  it('adopts only the samples a project does not already hold', () => {
    const edited = {
      ...samples[0]!,
      name: 'My own name',
      text: 'My own text',
    }
    const merged = withBuiltinSamples([edited], samples)
    expect(merged).toHaveLength(samples.length)
    expect(merged[0]).toBe(edited)
    expect(withBuiltinSamples(merged, samples)).toHaveLength(samples.length)
  })
})
