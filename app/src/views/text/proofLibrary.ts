/** Project metadata namespace; native saves already preserve FontInfo.user_data. */
export const PROOF_LIBRARY_KEY = 'com.fontcarver.proofLibrary'

/** Maximum library size and per-entry proof length (UTF-16 code units). */
export const MAX_PROOF_ENTRIES = 256
export const MAX_PROOF_TEXT_LENGTH = 32_768

/** A named project proof, retaining literal text and explicit glyph escapes. */
export interface ProofEntry {
  id: string
  name: string
  group: string
  script: string
  language: string
  direction: 'ltr' | 'rtl' | 'ttb'
  text: string
}

/** Stable errors translated by the caller rather than shown as raw validators. */
export type ProofLibraryErrorCode =
  | 'invalidEntry'
  | 'invalidLibrary'
  | 'unsupportedVersion'
  | 'incompatibleUserData'
  | 'full'

/** A validation error that never replaces the original project metadata. */
export class ProofLibraryError extends Error {
  constructor(readonly code: ProofLibraryErrorCode) {
    super(code)
    this.name = 'ProofLibraryError'
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function boundedString(value: unknown, maximum: number): value is string {
  return typeof value === 'string' && value.length <= maximum
}

function checkedEntry(value: unknown): ProofEntry {
  if (
    !isRecord(value) ||
    typeof value.id !== 'string' ||
    !/^[a-zA-Z0-9][a-zA-Z0-9._:-]{0,95}$/.test(value.id) ||
    !boundedString(value.name, 128) ||
    value.name.trim() === '' ||
    !boundedString(value.group, 128) ||
    !boundedString(value.script, 64) ||
    !boundedString(value.language, 64) ||
    (value.direction !== 'ltr' &&
      value.direction !== 'rtl' &&
      value.direction !== 'ttb') ||
    !boundedString(value.text, MAX_PROOF_TEXT_LENGTH) ||
    value.text.length === 0
  ) {
    throw new ProofLibraryError('invalidEntry')
  }
  return {
    ...value,
    id: value.id,
    name: value.name,
    group: value.group,
    script: value.script,
    language: value.language,
    direction: value.direction,
    text: value.text,
  }
}

function checkedEntries(value: unknown): ProofEntry[] {
  if (!Array.isArray(value)) throw new ProofLibraryError('invalidLibrary')
  if (value.length > MAX_PROOF_ENTRIES) throw new ProofLibraryError('full')
  const entries = value.map(checkedEntry)
  if (new Set(entries.map((entry) => entry.id)).size !== entries.length) {
    throw new ProofLibraryError('invalidLibrary')
  }
  return entries
}

function namespace(userData: unknown): Record<string, unknown> | null {
  if (!isRecord(userData) || !Object.hasOwn(userData, PROOF_LIBRARY_KEY)) {
    return null
  }
  const library = userData[PROOF_LIBRARY_KEY]
  if (!isRecord(library)) throw new ProofLibraryError('invalidLibrary')
  if (library.version !== 1) throw new ProofLibraryError('unsupportedVersion')
  return library
}

/** Reads a bounded library and refuses malformed/future data without losing it. */
export function readProofLibrary(userData: unknown): ProofEntry[] {
  const library = namespace(userData)
  return library === null ? [] : checkedEntries(library.entries)
}

/** Returns new metadata while retaining all unrelated and future attributes. */
export function writeProofLibrary(
  userData: unknown,
  entries: readonly ProofEntry[],
): Record<string, unknown> {
  if (userData != null && !isRecord(userData)) {
    throw new ProofLibraryError('incompatibleUserData')
  }
  // Check what we are about to replace; a corrupt or newer library must remain
  // recoverable instead of disappearing the first time an entry is saved.
  readProofLibrary(userData)
  return {
    ...(userData ?? {}),
    [PROOF_LIBRARY_KEY]: {
      ...namespace(userData),
      version: 1,
      entries: checkedEntries(entries),
    },
  }
}

/** Optional metadata and free-text filters; empty fields match every entry. */
export interface ProofLibraryFilter {
  query?: string
  group?: string
  script?: string
  language?: string
}

/** Searches exact metadata filters and all case-insensitive query words. */
export function filterProofLibrary(
  entries: readonly ProofEntry[],
  filter: ProofLibraryFilter = {},
): ProofEntry[] {
  const words = (filter.query ?? '').trim().toLocaleLowerCase().split(/\s+/)
  return entries.filter((entry) => {
    for (const key of ['group', 'script', 'language'] as const) {
      if (filter[key] && entry[key] !== filter[key]) return false
    }
    const searchable = [
      entry.name,
      entry.group,
      entry.script,
      entry.language,
      entry.text,
    ]
      .join('\n')
      .toLocaleLowerCase()
    return words.every((word) => searchable.includes(word))
  })
}

/** Distinct named groups for the project's filter menu. */
export function proofLibraryGroups(entries: readonly ProofEntry[]): string[] {
  return [...new Set(entries.map((entry) => entry.group).filter(Boolean))].sort(
    (left, right) => left.localeCompare(right),
  )
}

/** Wraps through the visible proofs without depending on an array index. */
export function stepProofLibrary(
  entries: readonly ProofEntry[],
  currentId: string | null,
  direction: 1 | -1,
): ProofEntry | null {
  if (entries.length === 0) return null
  const current = entries.findIndex((entry) => entry.id === currentId)
  const next =
    current < 0
      ? direction === 1
        ? 0
        : entries.length - 1
      : (current + direction + entries.length) % entries.length
  return entries[next] ?? null
}
