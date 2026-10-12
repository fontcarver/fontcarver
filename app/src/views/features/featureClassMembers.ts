import type { FeatureClass, Features } from '../../api'

/** A source condition which prevents a complete, reliable class preview. */
export interface FeatureClassIssue {
  kind: 'missing' | 'cycle' | 'duplicate' | 'unsupported' | 'limit'
  name: string
}

/** Source-order, unique names; incomplete results always include an issue. */
export interface FeatureClassMembers {
  members: string[]
  issues: FeatureClassIssue[]
  truncated: boolean
}

const MAX_MEMBERS = 512
const MAX_CLASSES = 4096
const MAX_SOURCE_LENGTH = 131072
const MAX_DEPTH = 32
const MAX_ISSUES = 32
const glyphName = /^(?:[A-Za-z_][A-Za-z0-9_.*+:^|~]*|\.notdef|\.null)$/
const className = /^@[A-Za-z_][A-Za-z0-9_.]*$/

interface MemberRange {
  count: number
  at: (offset: number) => string
}

type Member = { glyph: string } | { reference: string } | { range: MemberRange }

function rangeMembers(first: string, last: string): MemberRange | null {
  if (first.length !== last.length || first >= last) return null
  let start = 0
  while (first[start] === last[start]) start += 1
  let end = first.length
  while (first[end - 1] === last[end - 1]) end -= 1
  const left = first.slice(start, end)
  const right = last.slice(start, end)
  const letters =
    end - start === 1 &&
    ((/^[A-Z]$/.test(left) && /^[A-Z]$/.test(right)) ||
      (/^[a-z]$/.test(left) && /^[a-z]$/.test(right)))
  if (letters) {
    return {
      count: right.charCodeAt(0) - left.charCodeAt(0) + 1,
      at: (offset) =>
        first.slice(0, start) +
        String.fromCharCode(left.charCodeAt(0) + offset) +
        first.slice(end),
    }
  }
  while (start > 0 && /[0-9]/.test(first[start - 1])) start -= 1
  while (end < first.length && /[0-9]/.test(first[end])) end += 1
  const low = first.slice(start, end)
  const high = last.slice(start, end)
  if (!/^\d{1,3}$/.test(low) || !/^\d{1,3}$/.test(high)) return null
  return {
    count: Number(high) - Number(low) + 1,
    at: (offset) =>
      first.slice(0, start) +
      String(Number(low) + offset).padStart(low.length, '0') +
      first.slice(end),
  }
}

function parseMembers(code: string): Member[] | null {
  // A conservative preview parser: predicates, CIDs and ambiguous compact
  // hyphens need the compiler's glyph map and are reported instead of guessed.
  let body = code.replace(/#[^\r\n]*/g, '').trim()
  if (body.startsWith('[') && body.endsWith(']'))
    body = body.slice(1, -1).trim()
  if (!body) return []
  const tokens = body.split(/\s+/)
  const members: Member[] = []
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index]
    if (className.test(token)) {
      members.push({ reference: token.slice(1) })
      continue
    }
    const name = token.replace(/^\\/, '')
    if (!glyphName.test(name)) return null
    if (tokens[index + 1] === '-') {
      const last = (tokens[index + 2] ?? '').replace(/^\\/, '')
      const range = glyphName.test(last) ? rangeMembers(name, last) : null
      if (!range) return null
      members.push({ range })
      index += 2
    } else {
      members.push({ glyph: name })
    }
  }
  return members
}

/** Resolves editable class bodies without compiling or changing the document. */
export function resolveFeatureClassMembers(
  source: Features,
  requestedName: string,
): FeatureClassMembers {
  const name = requestedName.replace(/^@/, '')
  const answer: FeatureClassMembers = {
    members: [],
    issues: [],
    truncated: false,
  }
  const definitions = new Map<string, FeatureClass | null>()
  const active = new Set<string>()
  const visited = new Set<string>()
  const names = new Set<string>()
  let sourceLength = 0
  let operations = 0
  const issue = (kind: FeatureClassIssue['kind'], className: string) => {
    if (kind === 'limit') answer.truncated = true
    if (
      answer.issues.length < MAX_ISSUES &&
      !answer.issues.some(
        (item) => item.kind === kind && item.name === className,
      )
    ) {
      answer.issues.push({ kind, name: className })
    }
  }
  if (source.classes.length > MAX_CLASSES) issue('limit', name)
  for (const definition of source.classes.slice(0, MAX_CLASSES)) {
    definitions.set(
      definition.name,
      definitions.has(definition.name) ? null : definition,
    )
  }
  const add = (member: string, owner: string) => {
    operations += 1
    if (operations > 8192) {
      issue('limit', owner)
      return false
    }
    if (names.has(member)) return true
    if (names.size === MAX_MEMBERS) {
      issue('limit', owner)
      return false
    }
    names.add(member)
    answer.members.push(member)
    return true
  }
  const visit = (className: string, depth: number) => {
    if (active.has(className)) {
      issue('cycle', className)
      return
    }
    if (visited.has(className)) return
    if (depth >= MAX_DEPTH) {
      issue('limit', className)
      return
    }
    visited.add(className)
    const definition = definitions.get(className)
    if (definition === undefined) {
      issue('missing', className)
      return
    }
    if (definition === null) {
      issue('duplicate', className)
      return
    }
    sourceLength += definition.code.length
    if (sourceLength > MAX_SOURCE_LENGTH) {
      issue('limit', className)
      return
    }
    const members = parseMembers(definition.code)
    if (!members) {
      issue('unsupported', className)
      return
    }
    active.add(className)
    for (const member of members) {
      if (operations > 8192) {
        issue('limit', className)
        break
      }
      if ('reference' in member) visit(member.reference, depth + 1)
      else if ('range' in member) {
        for (let offset = 0; offset < member.range.count; offset += 1) {
          if (!add(member.range.at(offset), className)) break
        }
      } else if (!add(member.glyph, className)) break
    }
    active.delete(className)
  }
  visit(name, 0)
  return answer
}
