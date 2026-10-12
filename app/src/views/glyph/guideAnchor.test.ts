import { describe, expect, it } from 'vitest'

import type { Guide } from '../../api'

import {
  anchorOrigin,
  inkEdgesOf,
  NO_INK_EDGES,
  reanchorGuide,
  resolvedGuideX,
} from './guideAnchor'

const edges = { left: 80, right: 520 }

function guide(overrides: Partial<Guide> = {}): Guide {
  return {
    x: 0,
    y: 0,
    angle: 90,
    name: null,
    locked: false,
    color: null,
    anchor: 'Origin',
    ...overrides,
  }
}

describe('guides anchored to a sidebearing', () => {
  it('measures an anchored offset from the matching ink edge', () => {
    expect(resolvedGuideX(guide({ x: 120 }), edges)).toBe(120)
    expect(resolvedGuideX(guide({ x: 30, anchor: 'Lsb' }), edges)).toBe(110)
    expect(resolvedGuideX(guide({ x: -30, anchor: 'Rsb' }), edges)).toBe(490)
  })

  it('treats a blank glyph as absolute, because it has no ink edge', () => {
    expect(anchorOrigin('Lsb', NO_INK_EDGES)).toBeNull()
    expect(anchorOrigin('Rsb', { left: 0, right: Number.NaN })).toBeNull()
    expect(resolvedGuideX(guide({ x: 42, anchor: 'Lsb' }), NO_INK_EDGES)).toBe(
      42,
    )
  })

  it('rebases the offset so changing the anchor never moves the guide', () => {
    const absolute = guide({ x: 110 })
    const toLsb = reanchorGuide(absolute, 'Lsb', edges)
    expect(toLsb).toMatchObject({ anchor: 'Lsb', x: 30 })
    expect(resolvedGuideX(toLsb, edges)).toBe(110)
    const toRsb = reanchorGuide(toLsb, 'Rsb', edges)
    expect(toRsb).toMatchObject({ anchor: 'Rsb', x: -410 })
    expect(resolvedGuideX(toRsb, edges)).toBe(110)
    const back = reanchorGuide(toRsb, 'Origin', edges)
    expect(back).toMatchObject({ anchor: 'Origin', x: 110 })
    // Reasking for the anchor it already has is not an edit.
    expect(reanchorGuide(back, 'Origin', edges)).toBe(back)
  })

  it('reads the ink edges out of the scene metrics', () => {
    expect(inkEdgesOf({ width: 600, lsb: 80, rsb: 80 })).toEqual({
      left: 80,
      right: 520,
    })
    expect(inkEdgesOf({ width: 600, lsb: null, rsb: null })).toEqual(
      NO_INK_EDGES,
    )
  })
})
