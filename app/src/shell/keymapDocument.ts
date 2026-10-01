import { parseShortcut } from './shortcutSyntax'

/** Explicit overrides; null disables the action's shipped binding. */
export type KeymapOverrides = Record<string, string | null>

/** Portable normalized representation used for comparison and export. */
export function canonicalShortcut(value: string): string | null {
  const parsed = parseShortcut(value)
  if (!parsed) return null
  const key = parsed.key.length === 1 ? parsed.key.toUpperCase() : parsed.key
  const names: Record<string, string> = {
    space: 'Space',
    enter: 'Enter',
    escape: 'Escape',
    delete: 'Delete',
    backspace: 'Backspace',
    arrowleft: 'ArrowLeft',
    arrowright: 'ArrowRight',
    arrowup: 'ArrowUp',
    arrowdown: 'ArrowDown',
    home: 'Home',
    end: 'End',
    pageup: 'PageUp',
    pagedown: 'PageDown',
    tab: 'Tab',
  }
  return [
    parsed.mod && 'Mod',
    parsed.alt && 'Alt',
    parsed.shift && 'Shift',
    names[key] ?? key.toUpperCase(),
  ]
    .filter(Boolean)
    .join('+')
}

/**
 * A keymap file: the explicit bindings, plus the chords whose conflict the user
 * has deliberately accepted.
 *
 * An acknowledged chord is stored as its canonical form. A reader that does not
 * know the field ignores it and still applies the bindings, which is why the
 * document stays at version 1.
 */
export interface KeymapDocument {
  readonly bindings: KeymapOverrides
  readonly allowConflicts: readonly string[]
}

/** Validate a versioned, bounded JSON file without executing imported content. */
export function parseKeymapFile(text: string): KeymapDocument {
  if (text.length > 262_144) throw new Error('shortcuts.error.tooLarge')
  let value: unknown
  try {
    value = JSON.parse(text)
  } catch {
    throw new Error('shortcuts.error.invalid')
  }
  if (
    !value ||
    typeof value !== 'object' ||
    !('version' in value) ||
    value.version !== 1 ||
    !('bindings' in value)
  )
    throw new Error('shortcuts.error.invalid')
  const bindings = value.bindings
  if (!bindings || typeof bindings !== 'object' || Array.isArray(bindings))
    throw new Error('shortcuts.error.invalid')
  const result: KeymapOverrides = {}
  const entries = Object.entries(bindings)
  if (entries.length > 2000) throw new Error('shortcuts.error.tooLarge')
  for (const [id, shortcut] of entries) {
    if (
      !/^[a-zA-Z][a-zA-Z0-9_.-]{0,127}$/.test(id) ||
      ['__proto__', 'constructor', 'prototype'].includes(id)
    )
      throw new Error('shortcuts.error.invalid')
    if (shortcut === null) {
      result[id] = null
      continue
    }
    if (typeof shortcut !== 'string' || shortcut.length > 80)
      throw new Error('shortcuts.error.invalid')
    const canonical = canonicalShortcut(shortcut)
    const parsed = parseShortcut(shortcut)
    // Tab and Escape must remain available for focus traversal and dismissal.
    if (
      !canonical ||
      !parsed ||
      parsed.key === 'tab' ||
      parsed.key === 'escape'
    )
      throw new Error('shortcuts.error.invalid')
    result[id] = canonical
  }
  return { bindings: result, allowConflicts: parseAllowConflicts(value) }
}

/** The accepted-conflict list of a parsed document, canonical and deduplicated. */
function parseAllowConflicts(value: object): readonly string[] {
  if (!('allowConflicts' in value) || value.allowConflicts === undefined)
    return []
  const list = value.allowConflicts
  if (!Array.isArray(list) || list.length > 2000)
    throw new Error('shortcuts.error.invalid')
  const canonical = new Set<string>()
  for (const entry of list) {
    if (typeof entry !== 'string' || entry.length > 80)
      throw new Error('shortcuts.error.invalid')
    const normalized = canonicalShortcut(entry)
    if (!normalized) throw new Error('shortcuts.error.invalid')
    canonical.add(normalized)
  }
  return [...canonical]
}

/** The bindings of a keymap file. Kept for callers that need nothing else. */
export function parseKeymapDocument(text: string): KeymapOverrides {
  return parseKeymapFile(text).bindings
}

/**
 * Serialize overrides only; unrelated command defaults remain forwards
 * compatible. `allowConflicts` is written only when the user accepted one, so a
 * map without accepted conflicts serializes exactly as it did before.
 */
export function serializeKeymap(
  overrides: KeymapOverrides,
  allowConflicts: readonly string[] = [],
): string {
  return JSON.stringify(
    allowConflicts.length > 0
      ? { version: 1, bindings: overrides, allowConflicts: [...allowConflicts] }
      : { version: 1, bindings: overrides },
    null,
    2,
  )
}
