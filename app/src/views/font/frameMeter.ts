/**
 * Development frame-time meter.
 *
 * Only active in a development build. It records `requestAnimationFrame`
 * deltas into a ring buffer and exposes a report on `globalThis` so a browser
 * agent can read the 95th percentile while scrolling a large font.
 */

const CAPACITY = 4_096

/** One frame-time summary in milliseconds. */
export interface FrameReport {
  count: number
  p50: number
  p95: number
  max: number
}

interface FrameMeterApi {
  reset: () => void
  report: () => FrameReport
}

function percentile(sorted: readonly number[], fraction: number): number {
  if (sorted.length === 0) return 0
  const index = Math.min(
    sorted.length - 1,
    Math.max(0, Math.ceil(fraction * sorted.length) - 1),
  )
  return sorted[index] ?? 0
}

/**
 * Starts the meter and returns a stop function. Outside development builds it
 * does nothing at all.
 */
export function startFrameMeter(): () => void {
  if (!import.meta.env.DEV || typeof requestAnimationFrame !== 'function')
    return () => undefined

  let samples: number[] = []
  let previous = performance.now()
  let handle = 0
  let running = true

  const tick = (now: number) => {
    if (!running) return
    const delta = now - previous
    previous = now
    if (delta > 0 && delta < 2_000) {
      samples.push(delta)
      if (samples.length > CAPACITY) samples = samples.slice(-CAPACITY)
    }
    handle = requestAnimationFrame(tick)
  }
  handle = requestAnimationFrame(tick)

  const api: FrameMeterApi = {
    reset: () => {
      samples = []
      previous = performance.now()
    },
    report: () => {
      const sorted = [...samples].sort((left, right) => left - right)
      return {
        count: sorted.length,
        p50: Math.round(percentile(sorted, 0.5) * 100) / 100,
        p95: Math.round(percentile(sorted, 0.95) * 100) / 100,
        max: Math.round((sorted[sorted.length - 1] ?? 0) * 100) / 100,
      }
    },
  }
  ;(globalThis as { __fwFrameMeter?: FrameMeterApi }).__fwFrameMeter = api

  return () => {
    running = false
    cancelAnimationFrame(handle)
    delete (globalThis as { __fwFrameMeter?: FrameMeterApi }).__fwFrameMeter
  }
}
