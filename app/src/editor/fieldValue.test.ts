import { describe, expect, it } from 'vitest'

import { evaluateField, formatUnits } from './fieldValue'

describe('evaluateField', () => {
  it('replaces the value with a plain number', () => {
    expect(evaluateField('120', 40)).toBe(120)
    expect(evaluateField('-12.5', 40)).toBe(-12.5)
  })

  it('applies a relative expression to the current value', () => {
    expect(evaluateField('+10', 40)).toBe(50)
    expect(evaluateField('+-10', 40)).toBe(30)
    expect(evaluateField('*2', 40)).toBe(80)
    expect(evaluateField('/2', 40)).toBe(20)
  })

  it('rejects nonsense and division by zero', () => {
    expect(evaluateField('', 40)).toBeNull()
    expect(evaluateField('abc', 40)).toBeNull()
    expect(evaluateField('/0', 40)).toBeNull()
    expect(evaluateField('++4', 40)).toBeNull()
  })
})

describe('formatUnits', () => {
  it('keeps integers plain and rounds to one decimal', () => {
    expect(formatUnits(120)).toBe('120')
    expect(formatUnits(120.44)).toBe('120.4')
    expect(formatUnits(-0.04)).toBe('0')
  })
})
