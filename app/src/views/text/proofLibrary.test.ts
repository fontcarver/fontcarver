import { describe, expect, it } from 'vitest'
import {
  filterProofLibrary,
  MAX_PROOF_ENTRIES,
  MAX_PROOF_TEXT_LENGTH,
  PROOF_LIBRARY_KEY,
  proofLibraryGroups,
  readProofLibrary,
  stepProofLibrary,
  writeProofLibrary,
  type ProofEntry,
} from './proofLibrary'

const latin: ProofEntry = {
  id: 'latin-1',
  name: 'Latin spacing',
  group: 'Spacing',
  script: 'Latn',
  language: 'en',
  direction: 'ltr',
  text: 'AV /A /V /A.alt\r\n🪶',
}
const arabic: ProofEntry = {
  id: 'arabic-1',
  name: 'Arabic joining',
  group: 'Joining',
  script: 'Arab',
  language: 'ar',
  direction: 'rtl',
  text: 'العربية',
}
const cjk: ProofEntry = {
  id: 'cjk-1',
  name: '漢字',
  group: 'Spacing',
  script: 'Hani',
  language: 'ja',
  direction: 'ttb',
  text: '天地玄黃。',
}

describe('project proof library', () => {
  it('preserves exact text, unrelated data and future namespace attributes', () => {
    const source = {
      vendor: { nested: [1, 'unchanged'] },
      [PROOF_LIBRARY_KEY]: { version: 1, future: true, entries: [] },
    }
    const written = writeProofLibrary(source, [latin, arabic, cjk])
    expect(readProofLibrary(written)).toEqual([latin, arabic, cjk])
    expect(written).toMatchObject({
      vendor: source.vendor,
      [PROOF_LIBRARY_KEY]: { future: true },
    })
    expect(source[PROOF_LIBRARY_KEY].entries).toEqual([])
  })

  it('treats absent project metadata as an empty library', () => {
    for (const userData of [null, undefined, {}, { other: 'value' }]) {
      expect(readProofLibrary(userData)).toEqual([])
      expect(readProofLibrary(writeProofLibrary(userData, [latin]))).toEqual([
        latin,
      ])
    }
  })

  it('refuses replacing foreign scalar or array metadata', () => {
    for (const userData of [[], ['vendor'], 'value', 42, true]) {
      expect(() => writeProofLibrary(userData, [latin])).toThrow(
        'incompatibleUserData',
      )
    }
  })

  it('refuses malformed, duplicate or future library data without discarding it', () => {
    const invalid = [
      null,
      [],
      { version: 1 },
      { version: 1, entries: [latin, latin] },
      { version: 1, entries: [{ ...latin, direction: 'auto' }] },
      { version: 1, entries: [{ ...latin, direction: ['ltr'] }] },
    ]
    for (const value of invalid) {
      expect(() => readProofLibrary({ [PROOF_LIBRARY_KEY]: value })).toThrow()
      expect(() =>
        writeProofLibrary({ [PROOF_LIBRARY_KEY]: value }, []),
      ).toThrow()
    }
    expect(() =>
      readProofLibrary({ [PROOF_LIBRARY_KEY]: { version: 2, entries: [] } }),
    ).toThrow('unsupportedVersion')
  })

  it('bounds entries, identifiers and text instead of silently truncating', () => {
    expect(() => writeProofLibrary(null, [{ ...latin, id: 'bad id' }])).toThrow(
      'invalidEntry',
    )
    expect(() => writeProofLibrary(null, [{ ...latin, name: ' ' }])).toThrow(
      'invalidEntry',
    )
    expect(() => writeProofLibrary(null, [{ ...latin, text: '' }])).toThrow(
      'invalidEntry',
    )
    expect(() =>
      writeProofLibrary(null, [
        { ...latin, text: 'x'.repeat(MAX_PROOF_TEXT_LENGTH + 1) },
      ]),
    ).toThrow('invalidEntry')
    expect(() =>
      writeProofLibrary(
        null,
        Array.from({ length: MAX_PROOF_ENTRIES + 1 }, (_, index) => ({
          ...latin,
          id: `proof-${index}`,
        })),
      ),
    ).toThrow('full')
  })

  it('filters text and metadata together, lists unique groups and preserves order', () => {
    const entries = [latin, arabic, cjk]
    expect(filterProofLibrary(entries, { query: 'LATIN /A.ALT' })).toEqual([
      latin,
    ])
    expect(filterProofLibrary(entries, { group: 'Spacing' })).toEqual([
      latin,
      cjk,
    ])
    expect(
      filterProofLibrary(entries, { script: 'Arab', language: 'ar' }),
    ).toEqual([arabic])
    expect(
      filterProofLibrary(entries, { query: '漢字', group: 'Joining' }),
    ).toEqual([])
    expect(
      proofLibraryGroups([
        ...entries,
        { ...latin, id: 'ungrouped', group: '' },
      ]),
    ).toEqual(['Joining', 'Spacing'])
  })

  it('steps through a filtered list with wrapping and stable IDs', () => {
    const entries = [latin, cjk]
    expect(stepProofLibrary(entries, latin.id, 1)).toEqual(cjk)
    expect(stepProofLibrary(entries, latin.id, -1)).toEqual(cjk)
    expect(stepProofLibrary(entries, cjk.id, 1)).toEqual(latin)
    expect(stepProofLibrary(entries, 'missing', 1)).toEqual(latin)
    expect(stepProofLibrary(entries, null, -1)).toEqual(cjk)
    expect(stepProofLibrary([], null, 1)).toBeNull()
  })
})
