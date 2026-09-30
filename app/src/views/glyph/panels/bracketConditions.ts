import type { Axis } from '../../../api'
import { userToDesign } from '../../fontinfo/designCoordinates'

/** Validate inclusive design-space bounds before submitting a layer command. */
export function bracketConditionError(
  axis: Pick<Axis, 'min' | 'max' | 'map'> | undefined,
  min: number | null,
  max: number | null,
): string | null {
  if (min === null && max === null) return 'variation.conditionMissing'
  if (!axis) return 'variation.conditionMissing'
  const low = userToDesign(axis, axis.min)
  const high = userToDesign(axis, axis.max)
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
