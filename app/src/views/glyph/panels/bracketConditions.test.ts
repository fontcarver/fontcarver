import { expect, it } from 'vitest'
import { bracketConditionError } from './bracketConditions'

const axis = {
  min: 100,
  max: 900,
  map: [
    [100, 0],
    [900, 100],
  ] as [number, number][],
}
it('validates mapped design coordinates and inclusive single-value conditions', () => {
  expect(bracketConditionError(axis, 50, 50)).toBeNull()
  expect(bracketConditionError(axis, null, 100)).toBeNull()
  expect(bracketConditionError(axis, 0, null)).toBeNull()
  expect(bracketConditionError(axis, null, null)).toBe(
    'variation.conditionMissing',
  )
  expect(bracketConditionError(axis, 90, 10)).toBe(
    'variation.conditionReversed',
  )
  expect(bracketConditionError(axis, 200, null)).toBe(
    'variation.conditionInvalidRange',
  )
  expect(bracketConditionError(axis, Number.NaN, null)).toBe(
    'variation.conditionInvalidRange',
  )
})
