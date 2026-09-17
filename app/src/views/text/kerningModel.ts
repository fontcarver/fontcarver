import type {
  KerningResolution,
  KerningStatus,
  KernPair,
  KernSide,
} from '../../api'
import type { KerningGroupImpact } from './kerningImpact'

/** Concrete document glyph names, independent of compiled glyph IDs. */
export interface KerningPairNames {
  left: string
  right: string
}
/** Stored rule scopes in exception precedence order. */
export type KerningScope =
  'glyph_glyph' | 'glyph_group' | 'group_glyph' | 'group_group'
export const KERNING_SCOPES: readonly KerningScope[] = [
  'glyph_glyph',
  'glyph_group',
  'group_glyph',
  'group_group',
]

/** Derives the stored side identities, or null for missing memberships. */
export function kerningScopeSides(
  resolution: KerningResolution,
  scope: KerningScope,
): { left: KernSide; right: KernSide } | null {
  const [first, second] = scope.split('_')
  if (
    (first === 'group' && !resolution.left_group) ||
    (second === 'group' && !resolution.right_group)
  )
    return null
  return {
    left:
      first === 'group'
        ? { group: resolution.left_group! }
        : { glyph: resolution.left },
    right:
      second === 'group'
        ? { group: resolution.right_group! }
        : { glyph: resolution.right },
  }
}

/** Human-readable source identifiers; only names, with no implied compiled spacing. */
export function kerningSideName(side: KernSide): string {
  return 'glyph' in side ? side.glyph : side.group
}

/** Finds the exact record instead of mistaking inherited values for exceptions. */
export function kerningScopeRecord(
  resolution: KerningResolution,
  scope: KerningScope,
): KernPair | null {
  const sides = kerningScopeSides(resolution, scope)
  return sides
    ? (resolution.records.find(
        (record) =>
          JSON.stringify(record.left) === JSON.stringify(sides.left) &&
          JSON.stringify(record.right) === JSON.stringify(sides.right),
      ) ?? null)
    : null
}

/** Numeric field syntax accepts signed decimal font units and rejects blanks. */
export function parseKerningValue(text: string): number | null {
  const trimmed = text.trim()
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/u.test(trimmed)) return null
  const value = Number(trimmed)
  return Number.isFinite(value) ? value : null
}

/** Public panel model, kept independent of canvas selection and shaping. */
export interface TextKerningModel {
  pair: KerningPairNames
  scope: KerningScope
  status: KerningStatus | null
  resolution: KerningResolution | null
  loading: boolean
  loadFailed: boolean
  editable: boolean
  value: string
  invalid: boolean
  editing: boolean
  impact: KerningGroupImpact[]
  impactLoading: boolean
  impactFailed: boolean
  setPair: (pair: KerningPairNames) => Promise<void>
  setScope: (scope: KerningScope) => Promise<void>
  setValue: (value: string) => void
  commit: () => Promise<void>
  cancel: () => void
  remove: () => Promise<void>
}
