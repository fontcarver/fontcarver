import { beforeEach, describe, expect, it } from 'vitest'

import {
  initialEditorState,
  overlayEnabled,
  useEditorStore,
} from './editorStore'

beforeEach(() => useEditorStore.setState(initialEditorState()))

describe('editorStore', () => {
  it('starts with every overlay except the fill preview', () => {
    const { overlays } = useEditorStore.getState()
    expect(overlays).toEqual({
      filled: false,
      points: true,
      metrics: true,
      anchors: true,
      guides: true,
      rulers: true,
    })
  })

  it('toggles one overlay without touching the others', () => {
    useEditorStore.getState().toggleOverlay('filled')
    expect(overlayEnabled('filled')).toBe(true)
    expect(overlayEnabled('points')).toBe(true)

    useEditorStore.getState().setOverlay('points', false)
    expect(overlayEnabled('points')).toBe(false)
    expect(overlayEnabled('filled')).toBe(true)
  })

  it('ignores a pointer update that would not move the readout', () => {
    const store = useEditorStore
    store.getState().setPointer({ x: 10, y: 20 })
    const first = store.getState().pointer
    store.getState().setPointer({ x: 10, y: 20 })
    expect(store.getState().pointer).toBe(first)

    store.getState().setPointer({ x: 11, y: 20 })
    expect(store.getState().pointer).toEqual({ x: 11, y: 20 })
    store.getState().setPointer(null)
    expect(store.getState().pointer).toBeNull()
  })

  it('ignores a zoom update that does not change the value', () => {
    useEditorStore.getState().setZoom(0.5)
    expect(useEditorStore.getState().zoom).toBe(0.5)
    useEditorStore.getState().setZoom(0.5)
    expect(useEditorStore.getState().zoom).toBe(0.5)
  })

  it('holds and clears the glyph the editor is showing', () => {
    expect(useEditorStore.getState().activeGlyph).toBeNull()
    useEditorStore.getState().setActiveGlyph({
      doc: 1,
      gid: 7,
      glyph: {} as never,
      scene: {} as never,
      cell: null,
    })
    expect(useEditorStore.getState().activeGlyph?.gid).toBe(7)
    useEditorStore.getState().setActiveGlyph(null)
    expect(useEditorStore.getState().activeGlyph).toBeNull()
  })
})
