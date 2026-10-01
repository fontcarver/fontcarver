/**
 * Plain-text interchange for the project proof library.
 *
 * One tab-separated line per entry, so a library can be kept in a repository,
 * reviewed in a diff and edited by hand. Identifiers are deliberately absent:
 * they belong to one project, and an imported entry is matched to an existing
 * one by its group and name instead, so re-importing a corrected file updates
 * the entries it names rather than duplicating them.
 */

import {
  checkedProofEntry,
  MAX_PROOF_ENTRIES,
  ProofLibraryError,
  type ProofEntry,
} from './proofLibrary'

/** First line of an exported file; imports accept it as a comment. */
export const PROOF_INTERCHANGE_HEADER = '# FontCarver proof library'
const COLUMN_HEADER = '# group\tname\tscript\tlanguage\tdirection\ttext'

const ESCAPES: Record<string, string> = {
  '\\': '\\\\',
  '\t': '\\t',
  '\n': '\\n',
  '\r': '\\r',
}
const UNESCAPES: Record<string, string> = {
  '\\': '\\',
  '#': '#',
  t: '\t',
  n: '\n',
  r: '\r',
}

/**
 * Escapes one field.
 *
 * A leading `#` is escaped as well: an entry whose group starts with one would
 * otherwise write a line the importer reads as a comment and skips, which would
 * lose that entry silently. `#` anywhere else needs no escape, so a hand-written
 * file stays readable.
 */
function escapeField(value: string): string {
  const escaped = value.replace(
    /[\\\t\n\r]/g,
    (character) => ESCAPES[character]!,
  )
  return escaped.startsWith('#') ? `\\${escaped}` : escaped
}

function unescapeField(value: string): string {
  let result = ''
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index]!
    if (character !== '\\') {
      result += character
      continue
    }
    const next = value[index + 1]
    const replacement = next === undefined ? undefined : UNESCAPES[next]
    if (replacement === undefined) throw new ProofLibraryError('invalidEntry')
    result += replacement
    index += 1
  }
  return result
}

/**
 * Reads a chosen file as UTF-8 text.
 *
 * `FileReader` rather than `Blob.text()`: it is the one reader present in every
 * environment this code runs in, including the test DOM, so the import path has
 * a single implementation that is actually exercised.
 */
export function readProofFile(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result ?? ''))
    reader.onerror = () =>
      reject(reader.error ?? new ProofLibraryError('invalidLibrary'))
    reader.readAsText(file, 'utf-8')
  })
}

/** Serialises a library as a tab-separated document with a comment header. */
export function exportProofText(entries: readonly ProofEntry[]): string {
  const lines = entries.map((entry) =>
    [
      entry.group,
      entry.name,
      entry.script,
      entry.language,
      entry.direction,
      entry.text,
    ]
      .map(escapeField)
      .join('\t'),
  )
  return [PROOF_INTERCHANGE_HEADER, COLUMN_HEADER, ...lines, ''].join('\n')
}

/**
 * Parses a tab-separated document into validated entries.
 *
 * An entry whose group and name match an existing one keeps that entry's
 * identifier, so the import updates it in place; everything else is new and is
 * given an identifier by `makeId`. The whole document is rejected — leaving the
 * stored library untouched — if any line is malformed, so a typo never imports
 * half a file.
 *
 * @throws ProofLibraryError when a line is malformed or the result is too large.
 */
export function importProofText(
  text: string,
  existing: readonly ProofEntry[],
  makeId: () => string,
): ProofEntry[] {
  if (text.length > MAX_PROOF_ENTRIES * 4_096)
    throw new ProofLibraryError('full')
  const byKey = new Map(
    existing.map((entry) => [`${entry.group}\u0000${entry.name}`, entry]),
  )
  const parsed: ProofEntry[] = []
  const seen = new Set<string>()
  for (const raw of text.split(/\r\n|\n|\r/)) {
    // Only a stray carriage return is stripped: trailing spaces belong to the
    // text, and a spacing string such as `HH HOH ` depends on keeping them.
    const line = raw.replace(/\r$/, '')
    // A line with no tab carries no fields, so whitespace alone is ignorable;
    // a real line starting with `#` arrives escaped as `\#`.
    if (!line.includes('\t') && line.trim() === '') continue
    if (line.startsWith('#')) continue
    const fields = line.split('\t')
    if (fields.length !== 6) throw new ProofLibraryError('invalidEntry')
    const [group, name, script, language, direction, body] = fields.map(
      unescapeField,
    ) as [string, string, string, string, string, string]
    const key = `${group}\u0000${name}`
    if (seen.has(key)) throw new ProofLibraryError('invalidLibrary')
    seen.add(key)
    parsed.push(
      checkedProofEntry({
        id: byKey.get(key)?.id ?? makeId(),
        name,
        group,
        script,
        language,
        direction,
        text: body,
      }),
    )
  }
  if (parsed.length === 0) throw new ProofLibraryError('invalidLibrary')
  const replaced = new Set(parsed.map((entry) => entry.id))
  const merged = [
    ...existing.filter((entry) => !replaced.has(entry.id)),
    ...parsed,
  ]
  if (merged.length > MAX_PROOF_ENTRIES) throw new ProofLibraryError('full')
  return merged
}
