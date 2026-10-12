import { describe, expect, it } from 'vitest'

import {
  axisStep,
  designDefault,
  formatAxisValue,
  userToDesign,
} from './designCoordinates'

describe('design coordinates', () => {
  it('maps user coordinates through the axis map and extrapolates the ends', () => {
    const axis = {
      map: [
        [400, 0],
        [700, 1000],
      ] as [number, number][],
    }
    expect(userToDesign(axis, 400)).toBe(0)
    expect(userToDesign(axis, 550)).toBe(500)
    expect(userToDesign(axis, 700)).toBe(1000)
    // Outside the map the slope is one, so the value keeps its distance.
    expect(userToDesign(axis, 300)).toBe(-100)
    expect(userToDesign(axis, 800)).toBe(1100)
    expect(userToDesign({ map: [] }, 42)).toBe(42)
    expect(
      designDefault({
        tag: 'wght',
        name: 'Weight',
        min: 400,
        default: 550,
        max: 700,
        map: axis.map,
        hidden: false,
      }),
    ).toBe(500)
  })
})

describe('axis stepping', () => {
  const weight = { min: 400, default: 400, max: 700 }

  it('steps a weight axis by one so arrow keys reach whole values', () => {
    const step = axisStep(weight, [400, 700])
    expect(step).toBe(1)
    // 100 presses from the default land exactly on 500, not near it.
    expect(weight.default + 100 * step).toBe(500)
  })

  it('refines the step until every named location is reachable', () => {
    const width = { min: 75, default: 100, max: 125 }
    expect(axisStep(width, [87.5, 100])).toBe(0.5)
    expect(axisStep({ min: 0, default: 0, max: 1 }, [0.25])).toBe(0.25)
    expect(axisStep({ min: 0, default: 0, max: 1 }, [0.07])).toBe(0.01)
  })

  it('falls back to a thousandth of the range for an unreachable location', () => {
    expect(axisStep({ min: 0, default: 0, max: 1000 }, [0.0004])).toBe(1)
    expect(axisStep({ min: 0, default: 0.0004, max: 1000 })).toBe(1)
    // A degenerate axis still has a usable step.
    expect(axisStep({ min: 400, default: 400, max: 400 })).toBe(1)
  })

  it('shows only the decimals the step needs', () => {
    expect(formatAxisValue(500, 1)).toBe('500')
    expect(formatAxisValue(87.5, 0.5)).toBe('87.5')
    expect(formatAxisValue(0.25, 0.01)).toBe('0.25')
  })
})
