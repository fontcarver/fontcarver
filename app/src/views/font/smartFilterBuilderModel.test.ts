import { describe, expect, it } from 'vitest'

import type { SmartFilterNode } from '../../api'
import { SMART_FILTER_FIELDS as GENERATED_FIELDS } from '../../api/generated/smartFilterFields'

import {
  defaultSmartRule,
  formatSmartFilter,
  SMART_FILTER_FIELDS,
  smartRuleOperators,
} from './smartFilterBuilderModel'

describe('smart filter builder expression formatting', () => {
  it('retains nested all, any and none groups and escapes literal values', () => {
    const root: SmartFilterNode = {
      kind: 'all',
      children: [
        { kind: 'predicate', field: 'script', operator: 'eq', value: 'han' },
        {
          kind: 'any',
          children: [
            { kind: 'predicate', field: 'paths', operator: 'gt', value: '0' },
            {
              kind: 'none',
              children: [
                {
                  kind: 'predicate',
                  field: 'name',
                  operator: 'contains',
                  value: 'A"\\.alt',
                },
              ],
            },
          ],
        },
      ],
    }
    expect(formatSmartFilter(root)).toBe(
      '(script:"han" and (paths>"0" or not (name~"A\\"\\\\.alt")))',
    )
  })

  it('encodes regex and comparison operators without interpreting their values', () => {
    expect(
      formatSmartFilter({
        kind: 'predicate',
        field: 'name',
        operator: 'regex',
        value: '^A\\.alt$',
      }),
    ).toBe('name=~"^A\\\\.alt$"')
    expect(
      formatSmartFilter({
        kind: 'predicate',
        field: 'width',
        operator: 'gte',
        value: '500',
      }),
    ).toBe('width>="500"')
  })

  it('offers exactly the vocabulary the kernel parses, with useful defaults', () => {
    // One source: the builder re-exports the generated list, so this asserts
    // that it is the generated list rather than a copy that can drift.
    expect(SMART_FILTER_FIELDS).toBe(GENERATED_FIELDS)
    expect(SMART_FILTER_FIELDS.map(({ field }) => field)).toContain('locked')
    expect(SMART_FILTER_FIELDS.map(({ field }) => field)).toContain(
      'is_placeholder',
    )
    // Every field is offered with operators and a default that compile.
    for (const { field, type } of SMART_FILTER_FIELDS) {
      const operators = smartRuleOperators(field)
      expect(operators.length).toBeGreaterThan(0)
      const rule = defaultSmartRule(field)
      expect(rule).toMatchObject({ kind: 'predicate', field })
      expect(operators).toContain(
        (rule as { operator: string }).operator as never,
      )
      if (type === 'boolean') expect(rule).toMatchObject({ value: 'true' })
    }
    expect(defaultSmartRule('has_hints')).toMatchObject({
      operator: 'eq',
      value: 'true',
    })
    expect(defaultSmartRule('paths')).toMatchObject({
      operator: 'gt',
      value: '0',
    })
    expect(defaultSmartRule('name')).toMatchObject({
      operator: 'contains',
      value: '',
    })
  })

  it('offers numeric comparisons and a scalar literal for Unicode rules', () => {
    expect(smartRuleOperators('unicode')).toEqual([
      'eq',
      'ne',
      'gt',
      'gte',
      'lt',
      'lte',
    ])
    expect(defaultSmartRule('unicode')).toEqual({
      kind: 'predicate',
      field: 'unicode',
      operator: 'eq',
      value: 'U+0041',
    })
  })
})
