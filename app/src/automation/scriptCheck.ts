import {
  SCRIPT_METHODS,
  type ScriptMethodInfo,
} from '../api/generated/scriptingManifest'

/** One problem found in a macro's `fc.call` literals before it runs. */
export interface ScriptProblem {
  /**
   * Errors block the run: a literal method name is parsed reliably, so an
   * unknown or withheld method is certainly a mistake. Parameter problems are
   * warnings, because the kernel rejects them anyway and the key scan is a
   * heuristic that must never block a valid macro.
   */
  severity: 'error' | 'warning'
  /** One-based line of the offending call. */
  line: number
  /** Translation key describing the problem. */
  key: string
  /** Interpolation values for the message. */
  values: Record<string, string>
}

/** A method the console can complete, with its accepted parameter keys. */
export type ScriptMethod = ScriptMethodInfo

const MAX_CALLS = 256

const byName = new Map(SCRIPT_METHODS.map((entry) => [entry.method, entry]))

/** The generated contract for one kernel method, when it exists. */
export function scriptMethod(method: string): ScriptMethod | undefined {
  return byName.get(method)
}

/** Methods whose name or parameter keys match a query, best prefix matches first. */
export function searchScriptMethods(query: string, limit = 20): ScriptMethod[] {
  const needle = query.trim().toLowerCase()
  const scriptable = SCRIPT_METHODS.filter((entry) => entry.scriptable)
  if (needle === '') return scriptable.slice(0, limit)
  const scored = scriptable
    .map((entry) => {
      const name = entry.method.toLowerCase()
      if (name.startsWith(needle)) return { entry, rank: 0 }
      if (name.includes(needle)) return { entry, rank: 1 }
      if (entry.fields.some((field) => field.name.includes(needle)))
        return { entry, rank: 2 }
      return null
    })
    .filter((match): match is { entry: ScriptMethod; rank: number } => !!match)
  scored.sort(
    (a, b) => a.rank - b.rank || a.entry.method.localeCompare(b.entry.method),
  )
  return scored.slice(0, limit).map((match) => match.entry)
}

/**
 * A ready-to-run call template.
 *
 * It lists the keys the generated type marks as non-nullable, which is the best
 * available approximation of "required"; every other key can be added from the
 * method's parameter list beside it.
 */
export function callSnippet(method: ScriptMethod): string {
  const required = method.fields.filter((field) => !field.optional)
  const body = required.map((field) => `${field.name}: null`).join(', ')
  return `await fc.call('${method.method}', {${body ? ` ${body} ` : ''}})`
}

interface CallSite {
  line: number
  method: string
  keys: string[] | null
  spread: boolean
}

function skipString(source: string, start: number): number {
  const quote = source[start]
  for (let index = start + 1; index < source.length; index += 1) {
    const character = source[index]
    if (character === '\\') {
      index += 1
      continue
    }
    if (character === quote) return index + 1
    if (quote !== '`' && character === '\n') return index
  }
  return source.length
}

function skipComment(source: string, start: number): number {
  if (source[start + 1] === '/') {
    const end = source.indexOf('\n', start)
    return end === -1 ? source.length : end
  }
  const end = source.indexOf('*/', start)
  return end === -1 ? source.length : end + 2
}

/** Reads a string literal's value, or null when it is not a plain literal. */
function literal(
  source: string,
  start: number,
): { value: string; end: number } | null {
  const quote = source[start]
  if (quote !== "'" && quote !== '"' && quote !== '`') return null
  let value = ''
  for (let index = start + 1; index < source.length; index += 1) {
    const character = source[index]
    if (character === '\\') {
      value += source[index + 1] ?? ''
      index += 1
      continue
    }
    if (character === quote) return { value, end: index + 1 }
    if (character === '\n' || (quote === '`' && character === '$')) return null
    value += character
  }
  return null
}

/**
 * Collects the top-level keys of an object literal starting at `{`.
 *
 * `expectKey` keeps the scan honest: an identifier is only recorded where a
 * property name can appear, so a bare identifier used as a value is not
 * mistaken for a key. Returns null for an unterminated literal.
 */
function objectKeys(
  source: string,
  start: number,
): { keys: string[]; spread: boolean } | null {
  const keys: string[] = []
  let spread = false
  let depth = 0
  let index = start
  let expectKey = false
  while (index < source.length) {
    const character = source[index]
    if (character === '"' || character === "'" || character === '`') {
      if (depth === 1 && expectKey) {
        const text = literal(source, index)
        const after = text ? /^\s*:/.exec(source.slice(text.end)) : null
        if (text && after) {
          keys.push(text.value)
          index = text.end + after[0].length
          expectKey = false
          continue
        }
      }
      index = skipString(source, index)
      continue
    }
    if (
      character === '/' &&
      (source[index + 1] === '/' || source[index + 1] === '*')
    ) {
      index = skipComment(source, index)
      continue
    }
    if (character === '{' || character === '[' || character === '(') {
      if (depth === 1 && expectKey && character === '[') {
        // A computed key cannot be checked; treat it like a spread.
        spread = true
        expectKey = false
      }
      // Entering the literal itself: the next token is a property name.
      if (depth === 0 && character === '{') expectKey = true
      depth += 1
      index += 1
      continue
    }
    if (character === '}' || character === ']' || character === ')') {
      depth -= 1
      if (depth === 0) return { keys, spread }
      index += 1
      continue
    }
    if (depth === 1) {
      if (character === ',') {
        expectKey = true
        index += 1
        continue
      }
      if (expectKey) {
        if (source.startsWith('...', index)) {
          spread = true
          expectKey = false
          index += 3
          continue
        }
        // `key: value`, or the shorthand `key` before a comma or the brace.
        const property = /^([A-Za-z_$][\w$]*)(\s*)([:,}])/.exec(
          source.slice(index),
        )
        if (property) {
          keys.push(property[1])
          expectKey = property[3] === ','
          index +=
            property[3] === '}'
              ? property[1].length + property[2].length
              : property[0].length
          continue
        }
        if (!/\s/.test(character)) expectKey = false
      }
    }
    index += 1
  }
  return null
}

const CALL = /^(?:fc|fontcarver)\s*\.\s*call\s*\(/

/**
 * Finds every `fc.call`/`fontcarver.call` site whose method is a literal.
 *
 * The scan walks the source once, skipping string literals and comments, so a
 * commented-out or quoted call is never reported as a mistake.
 */
export function findCallSites(source: string): CallSite[] {
  const sites: CallSite[] = []
  let cursor = 0
  let line = 1
  while (cursor < source.length) {
    if (sites.length >= MAX_CALLS) break
    const character = source[cursor]
    if (character === '\n') {
      line += 1
      cursor += 1
      continue
    }
    if (character === '"' || character === "'" || character === '`') {
      const end = skipString(source, cursor)
      line += source.slice(cursor, end).split('\n').length - 1
      cursor = end
      continue
    }
    if (
      character === '/' &&
      (source[cursor + 1] === '/' || source[cursor + 1] === '*')
    ) {
      const end = skipComment(source, cursor)
      line += source.slice(cursor, end).split('\n').length - 1
      cursor = end
      continue
    }
    // A call must start at an identifier boundary, not inside another name.
    // The window keeps the scan linear in the source length.
    const boundary =
      (character === 'f' || character === 'F') &&
      (cursor === 0 || !/[\w$.]/.test(source[cursor - 1]))
    const match = boundary ? CALL.exec(source.slice(cursor, cursor + 48)) : null
    if (!match) {
      cursor += 1
      continue
    }
    const site = readCallSite(source, cursor + match[0].length, line)
    sites.push(site.value)
    line += source.slice(cursor, site.end).split('\n').length - 1
    cursor = site.end
  }
  return sites
}

function readCallSite(
  source: string,
  start: number,
  line: number,
): { value: CallSite; end: number } {
  let index = start
  while (/\s/.test(source[index] ?? '')) index += 1
  const method = literal(source, index)
  if (!method)
    return { value: { line, method: '', keys: null, spread: true }, end: index }
  index = method.end
  while (/\s/.test(source[index] ?? '')) index += 1
  if (source[index] !== ',')
    return {
      value: { line, method: method.value, keys: null, spread: false },
      end: index,
    }
  index += 1
  while (/\s/.test(source[index] ?? '')) index += 1
  if (source[index] !== '{')
    return {
      value: { line, method: method.value, keys: null, spread: false },
      end: index,
    }
  const object = objectKeys(source, index)
  return {
    value: {
      line,
      method: method.value,
      keys: object?.keys ?? null,
      spread: object?.spread ?? true,
    },
    end: index + 1,
  }
}

/**
 * Checks a macro against the generated API contract before it runs.
 *
 * Only literal `fc.call` sites are checked; a computed method name or a spread
 * parameter object is reported as unchecked rather than guessed at.
 */
export function checkScript(source: string): ScriptProblem[] {
  const problems: ScriptProblem[] = []
  for (const site of findCallSites(source)) {
    if (site.method === '') continue
    const method = byName.get(site.method)
    if (!method) {
      problems.push({
        severity: 'error',
        line: site.line,
        key: 'automation.check.unknownMethod',
        values: { method: site.method },
      })
      continue
    }
    if (!method.scriptable) {
      problems.push({
        severity: 'error',
        line: site.line,
        key: 'automation.check.unavailableMethod',
        values: { method: site.method },
      })
      continue
    }
    if (!method.closed || site.keys === null) continue
    // Only unknown keys are reported. A *missing* key cannot be judged here:
    // many kernel parameters are `#[serde(default)]`, which the generated
    // TypeScript does not distinguish from a required field, so the check
    // would reject valid calls.
    const accepted = new Set(method.fields.map((field) => field.name))
    for (const key of site.keys)
      if (!accepted.has(key))
        problems.push({
          severity: 'warning',
          line: site.line,
          key: 'automation.check.unknownParameter',
          values: { method: site.method, parameter: key },
        })
  }
  return problems
}
