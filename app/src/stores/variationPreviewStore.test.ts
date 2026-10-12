import { expect, it } from 'vitest'
import { useVariationPreviewStore } from './variationPreviewStore'

it('scopes previews to their owner and refuses cleanup from a previous owner', () => {
  const state = useVariationPreviewStore.getState()
  const preview = { path: 'M0 0L100 0Z', width: 600 }
  state.show(1, 'A', preview)
  expect(useVariationPreviewStore.getState().preview).toEqual({
    doc: 1,
    glyph: 'A',
    ...preview,
  })
  state.show(2, 'B', preview)
  state.clear(1, 'A')
  expect(useVariationPreviewStore.getState().preview?.doc).toBe(2)
  state.clear(2, 'B')
  expect(useVariationPreviewStore.getState().preview).toBeNull()
})
