import { describe, expect, it } from 'vitest'

import type { SmartFilterNode } from '../../api'

import {
  defaultSmartRule,
  formatSmartFilter,
  SMART_FILTER_FIELDS,
  smartRuleOperators,
} from './smartFilterBuilder'

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

  it('offers the complete predicate vocabulary with useful defaults', () => {
    expect(SMART_FILTER_FIELDS.map(({ field }) => field)).toEqual(
      expect.arrayContaining([
        'name',
        'paths',
        'components',
        'tags',
        'script',
        'category',
        'subcategory',
        'case',
        'master_compatible',
        'export',
        'has_unicode',
        'has_components',
        'contains_component',
        'has_hints',
        'has_ps_hints',
        'has_tt_hints',
        'has_corners',
        'has_anchors',
        'has_special_layers',
        'has_custom_glyph_info',
        'has_annotations',
        'is_auto_aligned',
        'has_metrics_keys',
        'metrics_out_of_sync',
        'has_kerning_groups',
        'color_label',
        'layer_color_label',
        'is_hangul_key',
      ]),
    )
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
