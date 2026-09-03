import { describe, expect, it } from 'vitest'

import { MAX_SCALE, MIN_SCALE, Viewport } from './Viewport'

function viewportOf(width = 800, height = 600): Viewport {
  const viewport = new Viewport()
  viewport.setSize(width, height, 2)
  return viewport
}

describe('Viewport', () => {
  it('maps font space y-up onto canvas space y-down', () => {
    const viewport = viewportOf()
    viewport.scale = 0.5
    viewport.offset = { x: 100, y: 400 }

    expect(viewport.toScreen({ x: 0, y: 0 })).toEqual({ x: 100, y: 400 })
    // A point above the baseline is nearer the top of the canvas.
    expect(viewport.toScreen({ x: 0, y: 200 })).toEqual({ x: 100, y: 300 })
    expect(viewport.toScreen({ x: 200, y: 0 })).toEqual({ x: 200, y: 400 })
  })

  it('round-trips a point through both spaces', () => {
    const viewport = viewportOf()
    viewport.scale = 0.37
    viewport.offset = { x: -42.5, y: 517.25 }

    for (const point of [
      { x: 0, y: 0 },
      { x: 1234.5, y: -321 },
      { x: -800, y: 2048 },
    ]) {
      const back = viewport.toFont(viewport.toScreen(point))
      expect(back.x).toBeCloseTo(point.x, 9)
      expect(back.y).toBeCloseTo(point.y, 9)
    }
  })

  it('fits a box with padding and centres it', () => {
    const viewport = viewportOf(800, 600)
    viewport.fit({ x0: 0, y0: -200, x1: 1000, y1: 800 }, 40)

    // The vertical extent is the binding one: 520 available px / 1000 units.
    expect(viewport.scale).toBeCloseTo(0.52, 9)
    const centre = viewport.toScreen({ x: 500, y: 300 })
    expect(centre.x).toBeCloseTo(400, 9)
    expect(centre.y).toBeCloseTo(300, 9)

    const topLeft = viewport.toScreen({ x: 0, y: 800 })
    expect(topLeft.y).toBeCloseTo(40, 9)
    const bottomRight = viewport.toScreen({ x: 1000, y: -200 })
    expect(bottomRight.y).toBeCloseTo(560, 9)
  })

  it('fits a degenerate box without dividing by zero', () => {
    const viewport = viewportOf(800, 600)
    viewport.fit({ x0: 300, y0: 0, x1: 300, y1: 0 }, 20)

    expect(Number.isFinite(viewport.scale)).toBe(true)
    expect(viewport.scale).toBe(MAX_SCALE)
    const origin = viewport.toScreen({ x: 300, y: 0 })
    expect(origin.x).toBeCloseTo(400, 9)
    expect(origin.y).toBeCloseTo(300, 9)
  })

  it('keeps the point under the cursor fixed while zooming', () => {
    const viewport = viewportOf()
    viewport.fit({ x0: 0, y0: -200, x1: 1000, y1: 800 }, 40)
    const cursor = { x: 613, y: 209 }
    const before = viewport.toFont(cursor)

    viewport.zoomAt(cursor, 1.6)
    const after = viewport.toFont(cursor)

    expect(viewport.scale).toBeCloseTo(0.52 * 1.6, 9)
    expect(after.x).toBeCloseTo(before.x, 9)
    expect(after.y).toBeCloseTo(before.y, 9)
  })

  it('clamps the scale at both ends', () => {
    const viewport = viewportOf()
    viewport.zoomAt({ x: 0, y: 0 }, 1e9)
    expect(viewport.scale).toBe(MAX_SCALE)
    viewport.zoomAt({ x: 0, y: 0 }, 1e-12)
    expect(viewport.scale).toBe(MIN_SCALE)
  })

  it('pans by a canvas-space delta', () => {
    const viewport = viewportOf()
    viewport.scale = 0.5
    viewport.offset = { x: 10, y: 20 }
    viewport.pan(-30, 15)
    expect(viewport.offset).toEqual({ x: -20, y: 35 })
  })

  it('zooms to an absolute scale around the canvas centre', () => {
    const viewport = viewportOf(800, 600)
    viewport.fit({ x0: 0, y0: 0, x1: 1000, y1: 1000 }, 0)
    const centre = viewport.toFont({ x: 400, y: 300 })

    viewport.zoomTo(1)

    expect(viewport.scale).toBe(1)
    const after = viewport.toFont({ x: 400, y: 300 })
    expect(after.x).toBeCloseTo(centre.x, 9)
    expect(after.y).toBeCloseTo(centre.y, 9)
  })

  it('records the canvas size and device pixel ratio', () => {
    const viewport = new Viewport()
    viewport.setSize(300, 200, 3)
    expect(viewport.width).toBe(300)
    expect(viewport.height).toBe(200)
    expect(viewport.dpr).toBe(3)
    viewport.setSize(-10, -10, 0)
    expect(viewport.width).toBe(0)
    expect(viewport.dpr).toBe(1)
  })
})
