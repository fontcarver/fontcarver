import { describe, expect, it } from 'vitest'

import {
  formatWaterfallSizes,
  MAX_WATERFALL_STEPS,
  parseWaterfallSizes,
  type WaterfallReason,
} from './waterfallSizes'

/** The accepted steps, or `null` when the entry was refused. */
function sizes(spec: string): number[] | null {
  return parseWaterfallSizes(spec).sizes
}

function reason(spec: string): WaterfallReason | null {
  return parseWaterfallSizes(spec).reason
}

describe('waterfall steps', () => {
  it('parses a comma or space separated list and sorts it ascending', () => {
    expect(sizes('12, 24, 48')).toEqual([12, 24, 48])
    expect(sizes('48 12 24')).toEqual([12, 24, 48])
    expect(sizes(' 10.5 , 9 ')).toEqual([9, 10.5])
  })

  it('parses a range with an explicit step in every accepted separator', () => {
    expect(sizes('8-32/8')).toEqual([8, 16, 24, 32])
    expect(sizes('8..32/8')).toEqual([8, 16, 24, 32])
    expect(sizes('8 to 32 / 8')).toEqual([8, 16, 24, 32])
  })

  it('stops a stepped range at the upper bound instead of overshooting it', () => {
    expect(sizes('10-25/7')).toEqual([10, 17, 24])
  })

  it('implies a step that fits the eight steps a waterfall shows', () => {
    const implied = sizes('8-72')
    expect(implied).toEqual([8, 18, 28, 38, 48, 58, 68])
    expect(implied!.length).toBeLessThanOrEqual(MAX_WATERFALL_STEPS)
    expect(sizes('12-18')).toEqual([12, 13, 14, 15, 16, 17, 18])
    expect(parseWaterfallSizes('8-72').truncated).toBe(false)
  })

  it('cuts a long range to the steps a waterfall shows and says it did', () => {
    // The documented example: nine steps asked for, eight drawn.
    const parsed = parseWaterfallSizes('8-72/8')
    expect(parsed.sizes).toEqual([8, 16, 24, 32, 40, 48, 56, 64])
    expect(parsed.sizes).toHaveLength(MAX_WATERFALL_STEPS)
    expect(parsed.truncated).toBe(true)
    expect(parsed.reason).toBeNull()
    expect(parseWaterfallSizes('6-256/1').sizes).toHaveLength(
      MAX_WATERFALL_STEPS,
    )
    expect(parseWaterfallSizes('6-256/1').truncated).toBe(true)
  })

  it('refuses an over-long list rather than shortening an explicit choice', () => {
    expect(sizes('1,2,3,4,5,6,7,8,9')).toBeNull()
    expect(reason('9,10,11,12,13,14,15,16,17')).toBe('count')
  })

  it('accepts a degenerate range as one step', () => {
    expect(sizes('24-24')).toEqual([24])
    expect(sizes('24-24/4')).toEqual([24])
  })

  it('names why an entry was refused', () => {
    expect(reason('')).toBe('empty')
    expect(reason('   ')).toBe('empty')
    expect(reason('12px, 24')).toBe('token')
    expect(reason('12, , 24')).toBe('token')
    expect(reason('-12, 24')).toBe('token')
    expect(reason('1e2')).toBe('token')
    expect(reason('NaN')).toBe('token')
    expect(reason('5, 24')).toBe('bounds')
    expect(reason('24, 257')).toBe('bounds')
    expect(reason('4-5/1')).toBe('bounds')
    expect(reason('24, 24')).toBe('duplicate')
    expect(reason('32-8')).toBe('range')
    expect(reason('8-32/0')).toBe('range')
    expect(reason('12, 24')).toBeNull()
  })

  it('round-trips an accepted list through its canonical text form', () => {
    const accepted = sizes('8-32/8')!
    expect(formatWaterfallSizes(accepted)).toBe('8, 16, 24, 32')
    expect(sizes(formatWaterfallSizes(accepted))).toEqual(accepted)
    // A truncated range round-trips as the list it actually drew.
    const cut = sizes('8-72/8')!
    expect(sizes(formatWaterfallSizes(cut))).toEqual(cut)
    expect(parseWaterfallSizes(formatWaterfallSizes(cut)).truncated).toBe(false)
  })
})
