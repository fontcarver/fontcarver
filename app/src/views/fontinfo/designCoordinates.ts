import type { EditableAxis as Axis } from './designSpaceSession'

/** Map an axis default into the model's design coordinate system. */
export function designDefault(axis: Axis): number {
  return userToDesign(axis, axis.default)
}

/** Map user coordinates, extrapolating unmapped ends with a unit slope. */
export function userToDesign(axis: Pick<Axis, 'map'>, value: number): number {
  const points = axis.map
  if (!points.length) return value
  const first = points[0]!,
    last = points[points.length - 1]!
  if (value <= first[0]) return value + first[1] - first[0]
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1]!,
      b = points[i]!
    if (value <= b[0])
      return a[1] + ((value - a[0]) / (b[0] - a[0])) * (b[1] - a[1])
  }
  return value + last[1] - last[0]
}

/** Steps a keyboard user can reach, coarsest first. */
const AXIS_STEPS = [1, 0.5, 0.25, 0.2, 0.1, 0.05, 0.01] as const

/**
 * The keyboard increment for an axis slider.
 *
 * `(max - min) / 1000` makes a weight axis step by 0.3, so no arrow key ever
 * lands on 500. Instead this picks the coarsest step that still reaches the
 * axis default, its maximum and every named location exactly — 1 for a
 * 400–700 weight axis, 0.5 for a width axis that carries 87.5.
 */
export function axisStep(
  axis: Pick<Axis, 'min' | 'default' | 'max'>,
  locations: readonly number[] = [],
): number {
  const offsets = [axis.default, axis.max, ...locations].map(
    (value) => value - axis.min,
  )
  const reachable = (step: number) =>
    offsets.every(
      (offset) => Math.abs(offset / step - Math.round(offset / step)) < 1e-6,
    )
  return AXIS_STEPS.find(reachable) ?? ((axis.max - axis.min) / 1000 || 1)
}

/** Show an axis coordinate with just the decimals its step needs. */
export function formatAxisValue(value: number, step: number): string {
  const decimals = step >= 1 ? 0 : step >= 0.1 ? 1 : 2
  return value.toFixed(decimals)
}
