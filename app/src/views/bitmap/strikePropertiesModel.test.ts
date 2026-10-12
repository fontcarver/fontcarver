import { describe, expect, it } from 'vitest'
import type { BdfValue } from '../../api'
import {
  isDerivedProperty,
  knownPropertyType,
  KNOWN_PROPERTIES,
  parsePropertyValue,
  propertyProblem,
  propertyText,
  propertyTypeOf,
  renameProperty,
} from './strikePropertiesModel'

describe('typed strike properties', () => {
  it('declares a type for every standard name and none for a custom one', () => {
    expect(knownPropertyType('RESOLUTION_X')).toBe('int')
    expect(knownPropertyType('FOUNDRY')).toBe('string')
    expect(knownPropertyType('MY_OWN_THING')).toBeNull()
    expect(new Set(KNOWN_PROPERTIES.map((entry) => entry.name)).size).toBe(
      KNOWN_PROPERTIES.length,
    )
    expect(isDerivedProperty('FONT_ASCENT')).toBe(true)
    expect(isDerivedProperty('FOUNDRY')).toBe(false)
  })

  it('rejects exactly what the formats cannot represent', () => {
    expect(propertyProblem('FOUNDRY', { Str: 'FontCarver' })).toBeNull()
    expect(propertyProblem('', { Int: 1 })).toBe('emptyName')
    expect(propertyProblem('A'.repeat(256), { Int: 1 })).toBe('nameLength')
    expect(propertyProblem('FAMILY NAME', { Int: 1 })).toBe('nameCharacters')
    expect(propertyProblem('TAB\tNAME', { Int: 1 })).toBe('nameCharacters')
    expect(propertyProblem('NOTE', { Str: 'one\ntwo' })).toBe('valueCharacters')
    expect(propertyProblem('NOTE', { Str: 'a'.repeat(65536) })).toBe(
      'valueLength',
    )
    expect(propertyProblem('POINT_SIZE', { Int: 1.5 })).toBe('notAnInteger')
    expect(propertyProblem('POINT_SIZE', { Int: 2 ** 60 })).toBe('integerRange')
    // A long multi-byte value is measured in bytes, as the formats store it.
    expect(propertyProblem('NOTE', { Str: '中'.repeat(21846) })).toBe(
      'valueLength',
    )
  })

  it('parses and prints each type without losing the text', () => {
    expect(parsePropertyValue('int', ' -12 ')).toEqual({ Int: -12 })
    expect(parsePropertyValue('int', '12.5')).toBeNull()
    expect(parsePropertyValue('int', 'twelve')).toBeNull()
    expect(parsePropertyValue('int', '')).toBeNull()
    expect(parsePropertyValue('string', ' spaced ')).toEqual({
      Str: ' spaced ',
    })
    expect(propertyText({ Int: -3 })).toBe('-3')
    expect(propertyText({ Str: 'x' })).toBe('x')
    expect(propertyTypeOf({ Int: 0 })).toBe('int')
    expect(propertyTypeOf({ Str: '' })).toBe('string')
  })

  it('renames in one step, refusing a collision or an unusable name', () => {
    const properties: Record<string, BdfValue> = {
      FOUNDRY: { Str: 'FontCarver' },
      POINT_SIZE: { Int: 80 },
    }
    const renamed = renameProperty(properties, 'FOUNDRY', 'VENDOR')
    expect(renamed).toEqual({
      POINT_SIZE: { Int: 80 },
      VENDOR: { Str: 'FontCarver' },
    })
    // The original map is untouched, so a failed write changes nothing.
    expect(Object.keys(properties)).toEqual(['FOUNDRY', 'POINT_SIZE'])
    expect(renameProperty(properties, 'FOUNDRY', 'POINT_SIZE')).toBe(
      'duplicate',
    )
    expect(renameProperty(properties, 'MISSING', 'X')).toBe('duplicate')
    expect(renameProperty(properties, 'FOUNDRY', 'BAD NAME')).toBe(
      'nameCharacters',
    )
    expect(renameProperty(properties, 'FOUNDRY', 'FOUNDRY')).toEqual(properties)
  })
})
