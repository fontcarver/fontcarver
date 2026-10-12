import { describe, expect, it } from 'vitest'

import type { MetricsKeys } from '../../../api'

import {
  effectiveMetricsKeys,
  metricsKeyCommand,
  metricsKeyError,
  normalizeMetricsKey,
} from './metricsKeyCommands'

/**
 * Formulas the kernel accepts. `crates/fc-doc/tests/metrics.rs` asserts the
 * same table through `SetMetricsKeys`, so the two grammars stay in step.
 */
export const VALID_METRICS_KEYS = [
  '=H',
  '=H+10',
  '=H-10',
  '=H*2',
  '=H/2',
  '=|H',
  '=|',
  '=50',
  '=-25',
  '=+25',
  '==H',
  '==50',
  '=H@200',
  '=|H@200',
  '=H+10@200',
  '=0.5',
  '=.5',
  // A trailing hyphen belongs to the glyph name, not to an arithmetic tail.
  '=H-',
]

/** Formulas the kernel refuses, with the reason the field reports. */
export const INVALID_METRICS_KEYS: readonly [string, string][] = [
  ['=', 'empty'],
  ['==', 'empty'],
  ['=+', 'offset'],
  ['=-x', 'offset'],
  ['=H@', 'height'],
  ['=H@x', 'height'],
  ['=H/0', 'divideByZero'],
  ['=H+', 'operand'],
  ['=H*', 'operand'],
  ['=H V', 'reference'],
  ['=H=V', 'reference'],
  ['=@200', 'empty'],
]

const keys: MetricsKeys = {
  left: '=H',
  right: '=n',
  width: null,
  vertical_origin: '=ideograph',
  vertical_width: '=ideograph+10',
}
const local: MetricsKeys = {
  left: '==O',
  right: null,
  width: '==H+100',
  vertical_origin: null,
  vertical_width: '==ideograph',
}

describe('metrics key scopes', () => {
  it('uses per-field layer overrides and inherits remaining glyph keys', () => {
    expect(effectiveMetricsKeys(keys, local)).toEqual({
      ...keys,
      left: '==O',
      width: '==H+100',
      vertical_width: '==ideograph',
    })
  })

  it('writes double-equals to this layer while retaining its other overrides', () => {
    expect(
      metricsKeyCommand('a', 'layer-1', keys, local, 'left', ' ==n '),
    ).toEqual({
      SetLayerMetricsKeys: {
        glyph: 'a',
        layer: 'layer-1',
        keys: { ...local, left: '==n' },
      },
    })
  })

  it('edits the global key and clears only the corresponding local override atomically', () => {
    expect(
      metricsKeyCommand('a', 'layer-1', keys, local, 'left', '=n+20'),
    ).toEqual({
      Batch: [
        { SetMetricsKeys: { glyph: 'a', keys: { ...keys, left: '=n+20' } } },
        {
          SetLayerMetricsKeys: {
            glyph: 'a',
            layer: 'layer-1',
            keys: { ...local, left: null },
          },
        },
      ],
    })
  })

  it('clears the last local override to resume glyph inheritance', () => {
    const onlyLeft = {
      left: '==n',
      right: null,
      width: null,
      vertical_origin: null,
      vertical_width: null,
    }
    expect(
      metricsKeyCommand('a', 'layer-1', keys, onlyLeft, 'left', ''),
    ).toEqual({
      SetLayerMetricsKeys: { glyph: 'a', layer: 'layer-1', keys: null },
    })
  })

  it('does not freeze inherited glyph keys when creating the first local override', () => {
    expect(
      metricsKeyCommand('a', 'layer-1', keys, null, 'left', '==H'),
    ).toEqual({
      SetLayerMetricsKeys: {
        glyph: 'a',
        layer: 'layer-1',
        keys: {
          left: '==H',
          right: null,
          width: null,
          vertical_origin: null,
          vertical_width: null,
        },
      },
    })
  })
})

describe('metricsKeyError', () => {
  it('accepts every documented formula', () => {
    for (const key of VALID_METRICS_KEYS)
      expect([key, metricsKeyError(key)]).toEqual([key, null])
  })

  it('names the problem for every refused formula', () => {
    for (const [key, problem] of INVALID_METRICS_KEYS)
      expect([key, metricsKeyError(key)]).toEqual([key, problem])
  })

  it('treats an empty field and whitespace as clearing the formula', () => {
    expect(metricsKeyError('')).toBeNull()
    expect(metricsKeyError('   ')).toBeNull()
    expect(normalizeMetricsKey('   ')).toBe('')
  })

  it('accepts the bare spelling by adding the equals sign', () => {
    expect(normalizeMetricsKey('H')).toBe('=H')
    expect(normalizeMetricsKey(' 50 ')).toBe('=50')
    expect(normalizeMetricsKey('=H')).toBe('=H')
    expect(normalizeMetricsKey('==H')).toBe('==H')
    // The validator normalises first, so the bare spelling is simply valid.
    expect(metricsKeyError('H')).toBeNull()
    expect(metricsKeyError('H+')).toBe('operand')
  })
})
