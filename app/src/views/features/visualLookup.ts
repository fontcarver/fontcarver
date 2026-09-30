/** Supported visual rules are deliberately smaller than the complete FEA language. */
export const VISUAL_LOOKUP_KINDS = [
  'gsub-single',
  'gsub-multiple',
  'gsub-alternate',
  'gsub-ligature',
  'gsub-contextual',
  'gpos-single',
  'gpos-pair',
  'gpos-mark-base',
  'gpos-mark-mark',
] as const

export type VisualLookupKind = (typeof VISUAL_LOOKUP_KINDS)[number]
export interface VisualValueRecord {
  xPlacement: number
  yPlacement: number
  xAdvance: number
  yAdvance: number
}
export interface VisualAnchor {
  x: number
  y: number
}

/** Glyph lists contain whitespace-separated literal names, without FEA brackets. */
export type VisualLookup =
  | {
      kind: 'gsub-single' | 'gsub-multiple' | 'gsub-alternate' | 'gsub-ligature'
      from: string
      to: string
    }
  | {
      kind: 'gsub-contextual'
      before: string
      input: string
      after: string
      replacement: string
    }
  | { kind: 'gpos-single'; glyphs: string; value: VisualValueRecord }
  | {
      kind: 'gpos-pair'
      first: string
      second: string
      firstValue: VisualValueRecord
      secondValue: VisualValueRecord
    }
  | {
      kind: 'gpos-mark-base' | 'gpos-mark-mark'
      markGlyphs: string
      markClass: string
      markAnchor: VisualAnchor
      targetGlyphs: string
      targetAnchor: VisualAnchor
    }

export const MAX_VISUAL_GLYPHS = 256
export const VISUAL_GLYPH_NAME = /^[A-Za-z_.][A-Za-z0-9_.-]{0,62}$/
export const VISUAL_CLASS_NAME = /^[A-Za-z_][A-Za-z0-9_.-]{0,62}$/

/** Feature tags must occupy four ASCII bytes and be valid bare FEA identifiers. */
export function isValidFeatureTag(tag: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_]{3}$/.test(tag)
}

const words = (list: string): string[] =>
  list.trim().split(/\s+/).filter(Boolean)
const zeroValue = (): VisualValueRecord => ({
  xPlacement: 0,
  yPlacement: 0,
  xAdvance: 0,
  yAdvance: 0,
})

export function defaultVisualLookup(kind: VisualLookupKind): VisualLookup {
  switch (kind) {
    case 'gsub-single':
    case 'gsub-multiple':
    case 'gsub-alternate':
    case 'gsub-ligature':
      return { kind, from: '', to: '' }
    case 'gsub-contextual':
      return { kind, before: '', input: '', after: '', replacement: '' }
    case 'gpos-single':
      return { kind, glyphs: '', value: zeroValue() }
    case 'gpos-pair':
      return {
        kind,
        first: '',
        second: '',
        firstValue: zeroValue(),
        secondValue: zeroValue(),
      }
    case 'gpos-mark-base':
    case 'gpos-mark-mark':
      return {
        kind,
        markGlyphs: '',
        markClass: 'visual_mark',
        markAnchor: { x: 0, y: 0 },
        targetGlyphs: '',
        targetAnchor: { x: 0, y: 0 },
      }
  }
}

/** Validation codes identify fields for callers to translate into UI messages. */
export function validateVisualLookup(rule: VisualLookup): string[] {
  const errors: string[] = []
  const glyphs = (
    field: string,
    value: string,
    min = 1,
    max = MAX_VISUAL_GLYPHS,
  ) => {
    const names = words(value)
    if (names.length < min) errors.push(`${field}:required`)
    if (names.length > max) errors.push(`${field}:count`)
    if (names.some((name) => !VISUAL_GLYPH_NAME.test(name)))
      errors.push(`${field}:identifier`)
    return names.length
  }
  const numbers = (field: string, value: VisualValueRecord | VisualAnchor) => {
    if (
      Object.values(value).some(
        (number) =>
          !Number.isInteger(number) || number < -32768 || number > 32767,
      )
    )
      errors.push(`${field}:integerRange`)
  }
  switch (rule.kind) {
    case 'gsub-single': {
      const from = glyphs('from', rule.from)
      const to = glyphs('to', rule.to)
      if (to !== 1 && from !== to) errors.push('to:matchingCount')
      break
    }
    case 'gsub-multiple':
      glyphs('from', rule.from, 1, 1)
      glyphs('to', rule.to, 2)
      break
    case 'gsub-alternate':
      glyphs('from', rule.from, 1, 1)
      glyphs('to', rule.to)
      break
    case 'gsub-ligature':
      glyphs('from', rule.from, 2)
      glyphs('to', rule.to, 1, 1)
      break
    case 'gsub-contextual':
      glyphs('before', rule.before, 0)
      glyphs('input', rule.input, 1, 1)
      glyphs('after', rule.after, 0)
      glyphs('replacement', rule.replacement, 1, 1)
      break
    case 'gpos-single':
      glyphs('glyphs', rule.glyphs)
      numbers('value', rule.value)
      break
    case 'gpos-pair':
      glyphs('first', rule.first)
      glyphs('second', rule.second)
      numbers('firstValue', rule.firstValue)
      numbers('secondValue', rule.secondValue)
      break
    case 'gpos-mark-base':
    case 'gpos-mark-mark':
      glyphs('markGlyphs', rule.markGlyphs)
      glyphs('targetGlyphs', rule.targetGlyphs)
      if (!VISUAL_CLASS_NAME.test(rule.markClass))
        errors.push('markClass:identifier')
      numbers('markAnchor', rule.markAnchor)
      numbers('targetAnchor', rule.targetAnchor)
      break
  }
  return errors
}

const glyphSequence = (list: string): string =>
  words(list)
    .map((name) => `\\${name}`)
    .join(' ')
const glyphClass = (list: string): string => {
  const sequence = glyphSequence(list)
  return words(list).length > 1 ? `[${sequence}]` : sequence
}
const valueRecord = (value: VisualValueRecord): string =>
  `<${value.xPlacement} ${value.yPlacement} ${value.xAdvance} ${value.yAdvance}>`
const anchorRecord = (anchor: VisualAnchor): string =>
  `<anchor ${anchor.x} ${anchor.y}>`

/** Render drafts, including incomplete edits. Call validation before treating them as valid FEA. */
export function serializeVisualLookup(rule: VisualLookup): string {
  switch (rule.kind) {
    case 'gsub-single':
      return `sub ${glyphClass(rule.from)} by ${glyphClass(rule.to)};`
    case 'gsub-multiple':
    case 'gsub-ligature':
      return `sub ${glyphSequence(rule.from)} by ${glyphSequence(rule.to)};`
    case 'gsub-alternate':
      return `sub ${glyphSequence(rule.from)} from [${glyphSequence(rule.to)}];`
    case 'gsub-contextual': {
      const context = [
        glyphSequence(rule.before),
        `${glyphSequence(rule.input)}'`,
        glyphSequence(rule.after),
      ]
        .filter(Boolean)
        .join(' ')
      return `sub ${context} by ${glyphSequence(rule.replacement)};`
    }
    case 'gpos-single':
      return `pos ${glyphClass(rule.glyphs)} ${valueRecord(rule.value)};`
    case 'gpos-pair':
      return `pos ${glyphClass(rule.first)} ${valueRecord(rule.firstValue)} ${glyphClass(rule.second)} ${valueRecord(rule.secondValue)};`
    case 'gpos-mark-base':
    case 'gpos-mark-mark':
      return `markClass ${glyphClass(rule.markGlyphs)} ${anchorRecord(rule.markAnchor)} @${rule.markClass};\npos ${rule.kind === 'gpos-mark-base' ? 'base' : 'mark'} ${glyphClass(rule.targetGlyphs)} ${anchorRecord(rule.targetAnchor)} mark @${rule.markClass};`
  }
}

export { parseVisualLookup } from './visualLookupParser'
