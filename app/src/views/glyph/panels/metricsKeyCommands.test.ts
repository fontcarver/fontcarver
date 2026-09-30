import { describe, expect, it } from 'vitest'

import type { MetricsKeys } from '../../../api'

import { effectiveMetricsKeys, metricsKeyCommand } from './metricsKeyCommands'

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
