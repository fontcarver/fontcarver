import { describe, expect, it } from 'vitest'
import { makeMasterMetrics } from '../../test/fontFixtures'
import { resolveMasterMetric } from './metricInheritance'

describe('master metric inheritance', () => {
  it('resolves line metrics from the master and current font UPM', () => {
    const metrics = makeMasterMetrics({ ascender: 800, descender: -200 })
    expect(resolveMasterMetric(metrics, 'hhea_ascender', 1000)).toEqual({
      value: 1000,
      source: 'master',
      overridden: false,
    })
    expect(resolveMasterMetric(metrics, 'typo_ascender', 1000).value).toBe(800)
    expect(resolveMasterMetric(metrics, 'typo_line_gap', 1000).value).toBe(200)
    expect(resolveMasterMetric(metrics, 'win_descent', 1000).value).toBe(200)
    expect(
      resolveMasterMetric(metrics, 'default_vertical_advance', 2048),
    ).toEqual({
      value: 2048,
      source: 'font',
      overridden: false,
    })
  })

  it('preserves explicit zero values and uses typo line gap overrides downstream', () => {
    const metrics = makeMasterMetrics({
      ascender: 800,
      descender: -200,
      typo_line_gap: 0,
      hhea_descender: 0,
    })
    expect(resolveMasterMetric(metrics, 'hhea_descender', 1000)).toEqual({
      value: 0,
      source: 'master',
      overridden: true,
    })
    expect(resolveMasterMetric(metrics, 'hhea_ascender', 1000).value).toBe(800)
    expect(resolveMasterMetric(metrics, 'win_ascent', 1000).value).toBe(800)
  })

  it('derives again from changed source metrics after resetting an override', () => {
    const metrics = makeMasterMetrics({ ascender: 850, descender: -300 })
    expect(
      resolveMasterMetric(metrics, 'default_vertical_origin', 1000).value,
    ).toBe(850)
    expect(resolveMasterMetric(metrics, 'typo_line_gap', 1000).value).toBe(50)
    expect(resolveMasterMetric(metrics, 'vhea_line_gap', 1000)).toEqual({
      value: 0,
      source: 'automatic',
      overridden: false,
    })
  })
})
