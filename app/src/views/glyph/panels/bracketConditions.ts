import type { Axis, BracketCondition } from '../../../api'
import { userToDesign } from '../../fontinfo/designCoordinates'

/** Axis identity as the API reports it: four bytes, not a string. */
export function axisTag(tag: Axis['tag']): string {
  return String.fromCharCode(...tag)
}

/** Design-space bounds of one axis, as the kernel validates them. */
export function axisBounds(
  axis: Pick<Axis, 'min' | 'max' | 'map'>,
): [number, number] {
  return [userToDesign(axis, axis.min), userToDesign(axis, axis.max)]
}

/** Validate inclusive design-space bounds before submitting a layer command. */
export function bracketConditionError(
  axis: Pick<Axis, 'min' | 'max' | 'map'> | undefined,
  min: number | null,
  max: number | null,
): string | null {
  if (min === null && max === null) return 'variation.conditionMissing'
  if (!axis) return 'variation.conditionMissing'
  const [low, high] = axisBounds(axis)
  if (
    [min, max].some(
      (value) =>
        value !== null &&
        (!Number.isFinite(value) || value < low || value > high),
    )
  )
    return 'variation.conditionInvalidRange'
  if (min !== null && max !== null && min > max)
    return 'variation.conditionReversed'
  return null
}

/**
 * Validate a complete condition set.
 *
 * A set is a conjunction of per-axis ranges — the OpenType `ConditionSet`
 * model — so it needs at least one bound, may not repeat an axis, and every
 * range must lie inside its axis.
 */
export function conditionSetError(
  axes: readonly Axis[],
  conditions: readonly BracketCondition[],
): string | null {
  if (conditions.length === 0) return 'variation.conditionMissing'
  const tags = conditions.map((condition) => axisTag(condition.axis))
  if (new Set(tags).size !== tags.length) return 'variation.conditionRepeated'
  if (
    conditions.every(
      (condition) =>
        (condition.min ?? null) === null && (condition.max ?? null) === null,
    )
  )
    return 'variation.conditionMissing'
  for (const condition of conditions) {
    const axis = axes.find(
      (value) => axisTag(value.tag) === axisTag(condition.axis),
    )
    if (!axis) return 'variation.conditionMissing'
    const error = bracketConditionError(
      axis,
      condition.min ?? null,
      condition.max ?? null,
    )
    // An individual range without a bound is legal inside a set that has one.
    if (error && error !== 'variation.conditionMissing') return error
  }
  return null
}

/** The Glyphs-style label of one range, for example `80‹wght` or `25‹wdth‹50`. */
export function formatCondition(condition: BracketCondition): string {
  const tag = axisTag(condition.axis)
  const min = condition.min ?? null
  const max = condition.max ?? null
  if (min !== null && max !== null) return `${min}‹${tag}‹${max}`
  if (min !== null) return `${min}‹${tag}`
  if (max !== null) return `${tag}‹${max}`
  return tag
}

/** The Glyphs-style label of a whole set, for example `[80‹wght, 25‹wdth‹50]`. */
export function formatConditionSet(
  conditions: readonly BracketCondition[],
): string {
  return `[${conditions.map(formatCondition).join(', ')}]`
}

/** Sort a set by axis tag, the canonical order the kernel compares. */
export function canonicalConditions(
  conditions: readonly BracketCondition[],
): BracketCondition[] {
  return [...conditions].sort((left, right) =>
    axisTag(left.axis) < axisTag(right.axis)
      ? -1
      : axisTag(left.axis) > axisTag(right.axis)
        ? 1
        : 0,
  )
}

/** Split a stored layer role into its primary range and the remaining ones. */
export function conditionsOf(kind: {
  Bracket: {
    axis: Axis['tag']
    min: number | null
    max: number | null
    extra?: BracketCondition[]
  }
}): BracketCondition[] {
  return canonicalConditions([
    {
      axis: kind.Bracket.axis,
      min: kind.Bracket.min ?? undefined,
      max: kind.Bracket.max ?? undefined,
    },
    ...(kind.Bracket.extra ?? []),
  ])
}
