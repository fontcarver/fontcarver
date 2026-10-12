import type { SmartAxis } from '../../../api'

/** Effective local coordinate, including an omitted axis default. */
export function smartValue(
  axis: SmartAxis,
  values: Record<string, number>,
): number {
  return values[axis.name] ?? axis.default
}

/** Keep all parameter inputs inside the model's supported interval. */
export function clampSmart(axis: SmartAxis, value: number): number {
  return Math.max(axis.min, Math.min(axis.max, value))
}

/** Stable finite parameter value at a normalized handle position. */
export function smartAtFraction(axis: SmartAxis, fraction: number): number {
  return clampSmart(
    axis,
    axis.min + Math.max(0, Math.min(1, fraction)) * (axis.max - axis.min),
  )
}

/** Default coordinates can be omitted without losing meaning. */
export function canonicalSmart(
  axes: SmartAxis[],
  values: Record<string, number>,
): Record<string, number> {
  const result = { ...values }
  for (const axis of axes) {
    if (result[axis.name] === axis.default) delete result[axis.name]
  }
  return result
}

/** Compare semantic parameter values without depending on object key order. */
export function sameSmart(
  axes: SmartAxis[],
  a: Record<string, number>,
  b: Record<string, number>,
): boolean {
  return axes.every((axis) => smartValue(axis, a) === smartValue(axis, b))
}
