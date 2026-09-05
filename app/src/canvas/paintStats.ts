/**
 * Development-only paint timing.
 *
 * The acceptance target for the editor is "dragging a node is 60 fps on a
 * 300-point glyph" (design spec §13), which means a pointer-move must repaint
 * in well under 16 ms. Measuring it needs no profiler: the canvas already
 * knows when a frame starts and ends, so in a dev build it records the
 * durations on `globalThis.__fwPaint` where a browser session — or a human
 * with the console open — can read them back.
 *
 * The whole module compiles away to two `if (false)` branches in a production
 * build, and records nothing when `performance` is unavailable.
 */

/** Rolling window of frame durations, in milliseconds. */
export interface PaintStats {
  samples: number[]
  count: number
  last: number
  max: number
  /** Mean of the retained window. */
  mean: number
  /** 95th percentile of the retained window. */
  p95: number
  reset: () => void
}

const WINDOW = 240

declare global {
  var __fwPaint: PaintStats | undefined
}

function summarise(stats: PaintStats): void {
  const sorted = [...stats.samples].sort((a, b) => a - b)
  stats.mean =
    sorted.reduce((total, value) => total + value, 0) / (sorted.length || 1)
  stats.p95 =
    sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))] ?? 0
  stats.max = sorted[sorted.length - 1] ?? 0
}

/** The stats object, created on first use. */
export function paintStats(): PaintStats {
  globalThis.__fwPaint ??= {
    samples: [],
    count: 0,
    last: 0,
    max: 0,
    mean: 0,
    p95: 0,
    reset: () => {
      const stats = globalThis.__fwPaint
      if (!stats) return
      stats.samples = []
      stats.count = 0
      stats.last = 0
      stats.max = 0
      stats.mean = 0
      stats.p95 = 0
    },
  }
  return globalThis.__fwPaint
}

/** Records one frame duration. */
export function recordPaint(milliseconds: number): void {
  const stats = paintStats()
  stats.samples.push(milliseconds)
  if (stats.samples.length > WINDOW) stats.samples.shift()
  stats.count += 1
  stats.last = milliseconds
  summarise(stats)
}
