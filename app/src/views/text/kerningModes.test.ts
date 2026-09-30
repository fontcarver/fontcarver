import { afterEach, describe, expect, it } from 'vitest'
import { useUiStore } from '../../stores/uiStore'
import { initialTextState } from './textState'
import { kerningFeatures, spacingLockedForDocument } from './kerningModes'

afterEach(() => useUiStore.setState({ tabs: [], activeTab: null }))

describe('kerning modes', () => {
  it('overrides both kerning features only while off and preserves stored choices', () => {
    const features: Array<[string, boolean]> = [
      ['kern', true],
      ['liga', false],
    ]
    expect(kerningFeatures(features, 'off')).toEqual([
      ['kern', false],
      ['liga', false],
      ['vkrn', false],
    ])
    expect(features).toEqual([
      ['kern', true],
      ['liga', false],
    ])
    expect(kerningFeatures(features, 'on')).toBe(features)
    expect(kerningFeatures(features, 'locked')).toBe(features)
    expect(kerningFeatures([], 'off')).toEqual([
      ['kern', false],
      ['vkrn', false],
    ])
  })

  it('locks only the active document tab and defaults legacy tabs to on', () => {
    const textState = initialTextState('regular')
    expect(textState.kerningMode).toBe('on')
    const legacy = { ...textState, kerningMode: undefined }
    useUiStore.setState({
      tabs: [
        {
          id: 'a',
          doc: 1,
          kind: 'glyph',
          textState: { ...textState, kerningMode: 'locked' },
        },
        { id: 'b', doc: 1, kind: 'glyph', textState: legacy },
      ],
      activeTab: 'a',
    })
    expect(spacingLockedForDocument(1)).toBe(true)
    expect(spacingLockedForDocument(2)).toBe(false)
    useUiStore.getState().setActiveTab('b')
    expect(spacingLockedForDocument(1)).toBe(false)
  })
})
