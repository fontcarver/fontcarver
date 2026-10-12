import { describe, expect, it } from 'vitest'

import type { Component, ComponentView } from '../../api'
import {
  applyAffine,
  composeAffine,
  rotateAbout,
  IDENTITY,
} from '../../editor/affine'

import { componentDraws, parsePath } from './componentDraws'

function makeComponent(
  transform: [number, number, number, number, number, number],
): Component {
  return {
    base: 'acute',
    transform,
    anchor: null,
    auto_align: false,
    use_my_metrics: false,
    round_to_grid: false,
    smart: null,
    point_match: null,
  }
}

function makeView(
  transform: [number, number, number, number, number, number],
): ComponentView {
  return {
    component: makeComponent(transform),
    path: 'M0 0L10 0L10 10Z',
    bbox: [0, 0, 10, 10],
  }
}

describe('componentDraws', () => {
  it('leaves an unpreviewed component at the identity', () => {
    const source = [makeView([1, 0, 0, 1, 100, 0])]
    const draws = componentDraws(source, null, new Map())
    expect(draws[0]?.transform).toEqual(IDENTITY)
    expect(draws[0]?.path).not.toBeNull()
  })

  it('carries a translation as a translation', () => {
    const source = [makeView([1, 0, 0, 1, 100, 0])]
    const preview = [makeView([1, 0, 0, 1, 140, -10])]
    expect(componentDraws(source, preview, new Map())[0]?.transform).toEqual([
      1, 0, 0, 1, 40, -10,
    ])
  })

  it('places a quarter turn where the kernel will put it', () => {
    // The kernel composes `affine * component.transform`
    // (`crates/fc-doc/src/command/outline/points.rs`), which is what the
    // gesture previews; the canvas has to draw the *difference* against the
    // path the kernel already rendered under the stored transform.
    const stored: [number, number, number, number, number, number] = [
      1, 0, 0, 1, 100, 0,
    ]
    const turn = rotateAbout({ x: 0, y: 0 }, Math.PI / 2)
    const previewed = composeAffine(turn, stored) as [
      number,
      number,
      number,
      number,
      number,
      number,
    ]
    const draws = componentDraws(
      [makeView(stored)],
      [makeView(previewed)],
      new Map(),
    )
    const delta = draws[0]?.transform
    expect(delta).toBeDefined()
    // The delta is the turn itself, not a slide: a component origin at
    // (100, 0) is drawn at (0, 100), exactly where the committed transform
    // will put it.
    const origin = { x: stored[4], y: stored[5] }
    const drawn = applyAffine(delta as typeof turn, origin)
    expect(drawn.x).toBeCloseTo(previewed[4], 9)
    expect(drawn.y).toBeCloseTo(previewed[5], 9)
    expect(drawn.x).toBeCloseTo(0, 9)
    expect(drawn.y).toBeCloseTo(100, 9)
    // The linear part really turns; a dx/dy pair could not express it.
    expect((delta as number[])[0]).toBeCloseTo(0, 9)
    expect((delta as number[])[1]).toBeCloseTo(1, 9)
  })

  it('falls back to a translation for a singular stored transform', () => {
    const source = [makeView([0, 0, 0, 0, 50, 50])]
    const preview = [makeView([0, 0, 0, 0, 90, 50])]
    expect(componentDraws(source, preview, new Map())[0]?.transform).toEqual([
      1, 0, 0, 1, 40, 0,
    ])
  })

  it('parses each path once', () => {
    const cache = new Map<string, Path2D | null>()
    const first = parsePath('M0 0L1 1Z', cache)
    expect(parsePath('M0 0L1 1Z', cache)).toBe(first)
    expect(cache.size).toBe(1)
  })
})
