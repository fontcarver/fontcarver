import type { BdfValue } from '../../api'

/** Expected type of a known strike property. */
export type PropertyType = 'int' | 'string'

/**
 * The standard XLFD and BDF properties a strike can carry, with their type.
 *
 * The list is the typed vocabulary the editor offers and validates against; a
 * property outside it is a custom property, kept with whichever type the user
 * chooses. `FONT_ASCENT`, `FONT_DESCENT` and `PIXEL_SIZE` are derived from the
 * strike itself by the BDF writer, so they are marked derived and are not worth
 * storing by hand.
 */
export const KNOWN_PROPERTIES: readonly {
  name: string
  type: PropertyType
  derived?: boolean
}[] = [
  { name: 'FOUNDRY', type: 'string' },
  { name: 'FAMILY_NAME', type: 'string' },
  { name: 'WEIGHT_NAME', type: 'string' },
  { name: 'SLANT', type: 'string' },
  { name: 'SETWIDTH_NAME', type: 'string' },
  { name: 'ADD_STYLE_NAME', type: 'string' },
  { name: 'PIXEL_SIZE', type: 'int', derived: true },
  { name: 'POINT_SIZE', type: 'int' },
  { name: 'RESOLUTION_X', type: 'int' },
  { name: 'RESOLUTION_Y', type: 'int' },
  { name: 'SPACING', type: 'string' },
  { name: 'AVERAGE_WIDTH', type: 'int' },
  { name: 'CHARSET_REGISTRY', type: 'string' },
  { name: 'CHARSET_ENCODING', type: 'string' },
  { name: 'FONT_ASCENT', type: 'int', derived: true },
  { name: 'FONT_DESCENT', type: 'int', derived: true },
  { name: 'DEFAULT_CHAR', type: 'int' },
  { name: 'CAP_HEIGHT', type: 'int' },
  { name: 'X_HEIGHT', type: 'int' },
  { name: 'QUAD_WIDTH', type: 'int' },
  { name: 'UNDERLINE_POSITION', type: 'int' },
  { name: 'UNDERLINE_THICKNESS', type: 'int' },
  { name: 'SUPERSCRIPT_X', type: 'int' },
  { name: 'SUPERSCRIPT_Y', type: 'int' },
  { name: 'SUBSCRIPT_X', type: 'int' },
  { name: 'SUBSCRIPT_Y', type: 'int' },
  { name: 'MIN_SPACE', type: 'int' },
  { name: 'NORM_SPACE', type: 'int' },
  { name: 'MAX_SPACE', type: 'int' },
  { name: 'FIGURE_WIDTH', type: 'int' },
  { name: 'STRIKEOUT_ASCENT', type: 'int' },
  { name: 'STRIKEOUT_DESCENT', type: 'int' },
  { name: 'COPYRIGHT', type: 'string' },
  { name: 'NOTICE', type: 'string' },
  { name: 'FONT_VERSION', type: 'string' },
  { name: 'FULL_NAME', type: 'string' },
  { name: '_XMBDFED_INFO', type: 'string' },
]

/** The declared type of a known property, or null for a custom one. */
export function knownPropertyType(name: string): PropertyType | null {
  return KNOWN_PROPERTIES.find((entry) => entry.name === name)?.type ?? null
}

/** Whether the BDF writer derives this property from the strike itself. */
export function isDerivedProperty(name: string): boolean {
  return KNOWN_PROPERTIES.some(
    (entry) => entry.name === name && entry.derived === true,
  )
}

/** Why a property cannot be stored, or null when it is acceptable. */
export type PropertyProblem =
  | 'emptyName'
  | 'nameLength'
  | 'nameCharacters'
  | 'valueCharacters'
  | 'valueLength'
  | 'notAnInteger'
  | 'integerRange'

/**
 * The same rules the kernel enforces, applied before the request is sent.
 *
 * A name is one whitespace-free token and a string value is one line, because
 * that is all BDF, PCF and FNT can represent. Integer values are checked
 * against the model's signed 64-bit range.
 */
export function propertyProblem(
  name: string,
  value: BdfValue,
): PropertyProblem | null {
  if (!name) return 'emptyName'
  if ([...name].length > 255) return 'nameLength'
  if (/[\s\p{Cc}\p{Cf}]/u.test(name)) return 'nameCharacters'
  if ('Str' in value) {
    if (/[\n\r\0]/u.test(value.Str)) return 'valueCharacters'
    if (new TextEncoder().encode(value.Str).length > 65535) return 'valueLength'
    return null
  }
  if (!Number.isInteger(value.Int)) return 'notAnInteger'
  if (!Number.isSafeInteger(value.Int)) return 'integerRange'
  return null
}

/** Build a typed value from the editor's text, or null when it is unusable. */
export function parsePropertyValue(
  type: PropertyType,
  text: string,
): BdfValue | null {
  if (type === 'string') return { Str: text }
  const trimmed = text.trim()
  if (!/^[+-]?\d+$/.test(trimmed)) return null
  const value = Number(trimmed)
  return Number.isSafeInteger(value) ? { Int: value } : null
}

/** The editor's text for a stored value. */
export function propertyText(value: BdfValue): string {
  return 'Int' in value ? String(value.Int) : value.Str
}

/** The stored value's type. */
export function propertyTypeOf(value: BdfValue): PropertyType {
  return 'Int' in value ? 'int' : 'string'
}

/**
 * Rename one property while keeping the rest, rejecting a collision.
 *
 * A rename is a remove plus an insert in one write, so the strike never exists
 * with both names, and overwriting an unrelated property by accident is refused
 * rather than silently merged.
 */
export function renameProperty(
  properties: Record<string, BdfValue>,
  from: string,
  to: string,
): Record<string, BdfValue> | PropertyProblem | 'duplicate' {
  const value = properties[from]
  if (value === undefined) return 'duplicate'
  if (from === to) return { ...properties }
  if (Object.hasOwn(properties, to)) return 'duplicate'
  const problem = propertyProblem(to, value)
  if (problem) return problem
  const next: Record<string, BdfValue> = {}
  for (const [key, item] of Object.entries(properties))
    if (key !== from) next[key] = item
  next[to] = value
  return next
}
