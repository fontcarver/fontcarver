/** Waterfall step parsing shared by the preview strip and the proof sheet. */

/** Smallest and largest renderable step, and how many steps one waterfall holds. */
export const MIN_WATERFALL_SIZE = 6
export const MAX_WATERFALL_SIZE = 256
export const MAX_WATERFALL_STEPS = 8

/**
 * Why an entry was refused, as a message key suffix.
 *
 * The field says what is wrong with *this* entry rather than restating the whole
 * grammar, so a reader can correct it without re-reading the rules.
 */
export type WaterfallReason =
  'empty' | 'token' | 'range' | 'bounds' | 'duplicate' | 'count'

/** One parsed entry: the steps to draw, or the reason the entry was refused. */
export interface WaterfallSteps {
  /** Ascending unique steps, or `null` when the entry was refused. */
  sizes: number[] | null
  /** The refusal reason, or `null` when the entry was accepted. */
  reason: WaterfallReason | null
  /** True when a range was longer than a waterfall shows and was cut to fit. */
  truncated: boolean
}

const NUMBER = String.raw`\d+(?:\.\d+)?|\.\d+`
const RANGE = new RegExp(
  String.raw`^(${NUMBER})\s*(?:-|–|\.\.|to)\s*(${NUMBER})(?:\s*/\s*(${NUMBER}))?$`,
  'u',
)
const SINGLE = new RegExp(String.raw`^(?:${NUMBER})$`, 'u')

function normalise(value: number): number {
  return Number(value.toFixed(2))
}

function bounded(value: number): boolean {
  return (
    Number.isFinite(value) &&
    value >= MIN_WATERFALL_SIZE &&
    value <= MAX_WATERFALL_SIZE
  )
}

function refused(reason: WaterfallReason): WaterfallSteps {
  return { sizes: null, reason, truncated: false }
}

function accepted(sizes: number[], truncated: boolean): WaterfallSteps {
  return {
    sizes: [...sizes].sort((left, right) => left - right),
    reason: null,
    truncated,
  }
}

/**
 * The step a range without an explicit one uses.
 *
 * A bare range still has to fit the eight steps a waterfall shows, so the
 * implied step is the smallest whole number that does, and the resolved list is
 * always shown back to the user rather than left implicit.
 */
function impliedStep(from: number, to: number): number {
  return Math.max(1, Math.ceil((to - from) / (MAX_WATERFALL_STEPS - 1)))
}

/**
 * Parses either a list of steps (`12, 24, 48`) or a range with an optional step
 * size (`8-72/8`, `8..72`, `8 to 72`), returning ascending unique sizes.
 *
 * A **range** that would run past the eight steps a waterfall shows is cut to
 * the first eight and reported as truncated, because a range describes an
 * intention ("from here to there in these increments") that is still useful when
 * it does not fit. A **list** is an explicit choice, so too many entries are
 * refused rather than silently shortened. Every other refusal names its cause;
 * the caller keeps the previous list and shows that cause.
 */
export function parseWaterfallSizes(spec: string): WaterfallSteps {
  const trimmed = spec.trim()
  if (trimmed === '') return refused('empty')
  const range = RANGE.exec(trimmed)
  if (range) {
    const from = normalise(Number(range[1]))
    const to = normalise(Number(range[2]))
    const step =
      range[3] === undefined
        ? impliedStep(from, to)
        : normalise(Number(range[3]))
    if (!Number.isFinite(step) || step <= 0 || to < from)
      return refused('range')
    if (!bounded(from) || !bounded(to)) return refused('bounds')
    const sizes: number[] = []
    let truncated = false
    for (let size = from; size <= to + 1e-9; size = normalise(size + step)) {
      if (sizes.length === MAX_WATERFALL_STEPS) {
        truncated = true
        break
      }
      sizes.push(normalise(size))
    }
    if (sizes.length === 0) return refused('range')
    return accepted(sizes, truncated)
  }
  const parts = trimmed.split(/\s*,\s*|\s+/u)
  if (parts.some((part) => !SINGLE.test(part))) return refused('token')
  const sizes = parts.map((part) => normalise(Number(part)))
  if (!sizes.every(bounded)) return refused('bounds')
  if (new Set(sizes).size !== sizes.length) return refused('duplicate')
  if (sizes.length > MAX_WATERFALL_STEPS) return refused('count')
  return accepted(sizes, false)
}

/** The canonical list form, which is what an accepted range is replaced with. */
export function formatWaterfallSizes(sizes: readonly number[]): string {
  return sizes.map((size) => String(normalise(size))).join(', ')
}
