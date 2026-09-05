/**
 * Numeric field behaviour shared by the Info box and the inspector.
 *
 * Design spec §7: number fields accept expressions relative to the current
 * value — `+10`, `*2`, `/2`, and `+-10` to subtract — as well as a plain
 * replacement, and reject anything else so the caller can shake the field
 * instead of writing nonsense into the outline. A leading `-` is a negative
 * coordinate, not a subtraction: font space is full of them, and guessing
 * wrong would silently move an outline.
 */

/** Evaluates a field entry against its current value, or `null` when invalid. */
export function evaluateField(input: string, current: number): number | null {
  const text = input.trim().replace(/\s+/g, '')
  if (text.length === 0) return null
  const relative = /^([+*/])(-?\d+(?:\.\d+)?)$/.exec(text)
  if (relative) {
    const amount = Number(relative[2])
    if (!Number.isFinite(amount)) return null
    switch (relative[1]) {
      case '+':
        return current + amount
      case '*':
        return current * amount
      default:
        return amount === 0 ? null : current / amount
    }
  }
  const absolute = Number(text)
  return Number.isFinite(absolute) ? absolute : null
}

/** Renders a font-unit value for a field: integers plain, else one decimal. */
export function formatUnits(value: number): string {
  if (!Number.isFinite(value)) return '0'
  const rounded = Math.round(value * 10) / 10
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1)
}
