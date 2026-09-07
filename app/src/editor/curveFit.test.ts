import { describe, expect, it } from 'vitest'

import type { Point2 } from '../canvas/Viewport'

import {
  cubicToQuadratics,
  fitContour,
  fitSamples,
  findCorners,
} from './curveFit'
import {
  contourToPath,
  segmentPointAt,
  type GeometricPath,
} from './pathGeometry'

/** A deterministic pseudo-random sequence, so the fit is reproducible. */
function noise(seed: number): () => number {
  let state = seed
  return () => {
    state = (state * 1664525 + 1013904223) % 4294967296
    return state / 4294967296 - 0.5
  }
}

/**
 * ~200 samples around a circle, jittered by up to ±0.25 units per axis —
 * about the pointer noise a 2 CSS-pixel sampling step leaves at editing zoom,
 * and inside the one-unit tolerance the fit is asked for.
 */
function noisyCircle(radius = 200, count = 200, amplitude = 0.5): Point2[] {
  const jitter = noise(7)
  const points: Point2[] = []
  for (let index = 0; index <= count; index += 1) {
    const angle = (index / count) * Math.PI * 2
    points.push({
      x: Math.cos(angle) * radius + jitter() * amplitude,
      y: Math.sin(angle) * radius + jitter() * amplitude,
    })
  }
  return points
}

/**
 * The largest distance from any sample to the fitted path.
 *
 * The path is sampled finely enough that the measurement's own discretisation
 * stays well under the tolerance being asserted.
 */
function maxDeviation(path: GeometricPath, samples: readonly Point2[]): number {
  const dense: Point2[] = []
  for (const segment of path.segments)
    for (let step = 0; step <= 512; step += 1)
      dense.push(segmentPointAt(segment, step / 512))
  let worst = 0
  for (const sample of samples) {
    let nearest = Infinity
    for (const point of dense)
      nearest = Math.min(
        nearest,
        Math.hypot(point.x - sample.x, point.y - sample.y),
      )
    worst = Math.max(worst, nearest)
  }
  return worst
}

describe('fitSamples', () => {
  it('fits a noisy circle to a handful of segments inside the tolerance', () => {
    const samples = noisyCircle()
    const contour = fitContour(samples, true, 'cubic', 1)
    expect(contour).not.toBeNull()
    const points = contour?.points ?? []
    // 201 samples in, well under twenty stored points out.
    expect(points.length).toBeLessThan(20)
    expect(points.length).toBeGreaterThan(3)
    const path = contourToPath(contour as NonNullable<typeof contour>)
    expect(maxDeviation(path, samples)).toBeLessThan(1)
  })

  it('keeps a corner node where the stroke turns sharply', () => {
    const samples: Point2[] = []
    for (let x = 0; x <= 200; x += 4) samples.push({ x, y: 0 })
    for (let y = 4; y <= 200; y += 4) samples.push({ x: 200, y })
    const corners = findCorners(samples)
    expect(corners).toHaveLength(1)

    const contour = fitContour(samples, false, 'cubic', 1)
    const onCurve = (contour?.points ?? []).filter(
      (point) => point.kind !== 'OffCurve',
    )
    // Start, the corner and the end — and the corner is not smoothed away.
    expect(onCurve).toHaveLength(3)
    expect(onCurve[1]).toMatchObject({ x: 200, y: 0, smooth: false })
    expect(contour?.points.every((point) => point.kind !== 'OffCurve')).toBe(
      true,
    )
  })

  it('fits a straight run as a line rather than a flat curve', () => {
    const samples = Array.from({ length: 40 }, (_, index) => ({
      x: index * 5,
      y: 0,
    }))
    const contour = fitContour(samples, false, 'cubic', 1)
    expect(contour?.points.map((point) => point.kind)).toEqual(['Move', 'Line'])
  })

  it('honours a coarser tolerance with fewer segments', () => {
    // Noise the one-unit tolerance cannot absorb forces more segments; asking
    // for two units lets the same stroke through in far fewer.
    const rough = noisyCircle(200, 200, 2)
    const tight = fitSamples(rough, 1).segments.length
    const loose = fitSamples(rough, 2).segments.length
    expect(loose).toBeLessThan(tight)
  })

  it('marks the joins inside a smooth run smooth', () => {
    const samples = noisyCircle(400, 240)
    const fitted = fitSamples(samples, 1)
    expect(fitted.segments.length).toBeGreaterThan(1)
    // Every join but the last, which ends the run.
    expect(fitted.smooth.slice(0, -1).every(Boolean)).toBe(true)
    expect(fitted.smooth[fitted.smooth.length - 1]).toBe(false)
  })

  it('produces one quadratic control per segment on a quadratic layer', () => {
    const contour = fitContour(noisyCircle(), true, 'quadratic', 1)
    const kinds = (contour?.points ?? []).map((point) => point.kind)
    expect(kinds).toContain('QCurve')
    expect(kinds).not.toContain('Curve')
    // Exactly one control between consecutive on-curve points.
    let controls = 0
    for (const kind of kinds) {
      if (kind === 'OffCurve') controls += 1
      else {
        expect(controls).toBeLessThanOrEqual(1)
        controls = 0
      }
    }
  })

  it('splits a cubic until one quadratic is inside the tolerance', () => {
    const cubic = {
      kind: 'cubic' as const,
      from: { x: 0, y: 0 },
      to: { x: 300, y: 0 },
      controls: [
        { x: 0, y: 400 },
        { x: 300, y: 400 },
      ],
    }
    const loose = cubicToQuadratics(cubic, 50)
    const tight = cubicToQuadratics(cubic, 0.5)
    expect(loose.length).toBeLessThan(tight.length)
    expect(tight.every((part) => part.kind === 'quad')).toBe(true)
    // Endpoints are preserved across the chain.
    expect(tight[0]?.from).toEqual(cubic.from)
    expect(tight[tight.length - 1]?.to).toEqual(cubic.to)
  })

  it('returns nothing for a stroke of fewer than two distinct samples', () => {
    expect(fitContour([{ x: 4, y: 4 }], false, 'cubic')).toBeNull()
    expect(
      fitContour(
        [
          { x: 4, y: 4 },
          { x: 4, y: 4 },
        ],
        false,
        'cubic',
      ),
    ).toBeNull()
  })
  it('keeps a quadratic layer inside the same total tolerance', () => {
    // The stroke is fitted twice on a quadratic layer — samples to cubics,
    // cubics to quadratics — and the two errors add, so the budget is split
    // rather than spent twice.
    const samples = noisyCircle()
    const contour = fitContour(samples, true, 'quadratic', 1)
    const path = contourToPath(contour as NonNullable<typeof contour>)
    expect(maxDeviation(path, samples)).toBeLessThan(1)
  })
})
