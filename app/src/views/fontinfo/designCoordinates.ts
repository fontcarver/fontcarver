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
