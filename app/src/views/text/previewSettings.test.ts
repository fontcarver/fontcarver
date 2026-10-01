import { describe, expect, it } from 'vitest'

import type { TextTabState } from '../../stores/uiStore'
import {
  previewInstanceId,
  previewInstanceOptions,
  previewSettingsOf,
  previewSettingsPatch,
} from './previewSettings'
import { PREVIEW_SIZES } from './previewPaint'

function tab(overrides: Partial<TextTabState> = {}): TextTabState {
  return {
    text: 'A',
    activeIndex: 0,
    mode: 'text',
    master: 'regular',
    size: 128,
    features: [],
    direction: 'ltr',
    previewTheme: 'light',
    previewCollapsed: false,
    ...overrides,
  }
}

describe('preview strip settings', () => {
  it('uses the shipped defaults for a tab saved before the settings existed', () => {
    expect(previewSettingsOf(tab())).toEqual({
      flipHorizontal: false,
      flipVertical: false,
      blur: 0,
      mode: 'run',
      sizes: [...PREVIEW_SIZES],
      instance: 'master:regular',
    })
  })

  it('reads stored settings and bounds a stored blur radius', () => {
    expect(
      previewSettingsOf(
        tab({
          previewFlipHorizontal: true,
          previewFlipVertical: true,
          previewBlur: 9,
          previewMode: 'single',
          previewSizes: [10, 20],
          previewInstance: 2,
        }),
      ),
    ).toEqual({
      flipHorizontal: true,
      flipVertical: true,
      blur: 4,
      mode: 'single',
      sizes: [10, 20],
      instance: 'instance:2',
    })
    expect(previewSettingsOf(tab({ previewBlur: Number.NaN })).blur).toBe(0)
  })

  it('names the picker value for a master and for a named instance', () => {
    expect(previewInstanceId(tab())).toBe('master:regular')
    expect(previewInstanceId(tab({ master: null }))).toBe('master:')
    expect(previewInstanceId(tab({ previewInstance: 0 }))).toBe('instance:0')
  })

  it('moves the run to a chosen master and clears the instance', () => {
    expect(previewSettingsPatch({ instance: 'master:bold' })).toEqual({
      master: 'bold',
      previewInstance: null,
    })
  })

  it('changes only the preview for a chosen named instance', () => {
    expect(previewSettingsPatch({ instance: 'instance:3' })).toEqual({
      previewInstance: 3,
    })
  })

  it('ignores an unusable picker value instead of corrupting the tab', () => {
    expect(previewSettingsPatch({ instance: 'instance:-1' })).toEqual({})
    expect(previewSettingsPatch({ instance: 'instance:x' })).toEqual({})
    expect(previewSettingsPatch({ instance: 'other' })).toEqual({})
    expect(previewSettingsPatch({ instance: 'master:' })).toEqual({
      previewInstance: null,
    })
  })

  it('copies the waterfall and bounds the blur it stores', () => {
    const sizes = [12, 24]
    const patch = previewSettingsPatch({ sizes, blur: -3 })
    expect(patch.previewSizes).toEqual(sizes)
    expect(patch.previewSizes).not.toBe(sizes)
    expect(patch.previewBlur).toBe(0)
    expect(previewSettingsPatch({ blur: 7 }).previewBlur).toBe(4)
  })

  it('patches nothing for an empty change', () => {
    expect(previewSettingsPatch({})).toEqual({})
  })

  it('lists masters before instances with distinct identifiers', () => {
    expect(
      previewInstanceOptions(
        [
          { id: 'a', name: 'Regular' },
          { id: 'b', name: 'Bold' },
        ],
        [{ name: 'SemiBold' }],
      ),
    ).toEqual([
      { id: 'master:a', name: 'Regular', kind: 'master' },
      { id: 'master:b', name: 'Bold', kind: 'master' },
      { id: 'instance:0', name: 'SemiBold', kind: 'instance' },
    ])
  })
})
